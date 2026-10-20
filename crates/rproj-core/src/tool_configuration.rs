//! Shared, UI-independent single-target settings drafts and captured reviews.
use std::{collections::BTreeMap, path::PathBuf};

use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::catalog::tool_settings::{self, ConfigTarget, ConfigurableTool, SettingKind};

impl ConfigTarget {
    pub fn relative(&self) -> &'static str {
        match self {
            Self::ProjectToml { filename } => filename,
            Self::VsCodeSettings => ".vscode/settings.json",
        }
    }
}

impl SettingKind {
    pub fn accepts(&self, value: &Value) -> bool {
        match self {
            Self::Bool { .. } => value.is_boolean(),
            Self::Integer { .. } => value.as_i64().is_some(),
            Self::Choice { options, .. } => options.iter().any(|o| value.as_str() == Some(o.value)),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettingsDraft {
    pub current: Vec<Option<Value>>,
    pub changes: Vec<Option<Value>>,
}

impl SettingsDraft {
    pub fn new(current: Vec<Option<Value>>) -> Self {
        Self {
            changes: vec![None; current.len()],
            current,
        }
    }
    pub fn dirty(&self) -> bool {
        self.changes.iter().any(Option::is_some)
    }
    pub fn value(&self, index: usize) -> Option<&Value> {
        self.changes
            .get(index)?
            .as_ref()
            .or(self.current.get(index)?.as_ref())
    }
    pub fn set(&mut self, index: usize, value: Value) {
        if let (Some(current), Some(change)) =
            (self.current.get(index), self.changes.get_mut(index))
        {
            *change = (current.as_ref() != Some(&value)).then_some(value);
        }
    }
    pub fn revert(&mut self, index: usize) {
        if let Some(change) = self.changes.get_mut(index) {
            *change = None;
        }
    }
    pub fn validate(&self, tool: &ConfigurableTool) -> Result<()> {
        ensure!(
            self.current.len() == tool.settings.len() && self.changes.len() == tool.settings.len(),
            "Settings draft does not match its catalog target"
        );
        for ((setting, change), current) in
            tool.settings.iter().zip(&self.changes).zip(&self.current)
        {
            if let Some(value) = change {
                ensure!(
                    setting.kind.accepts(value),
                    "Invalid value for {}",
                    setting.display_key()
                );
                ensure!(
                    Some(value) != current.as_ref(),
                    "Unchanged value submitted for {}",
                    setting.display_key()
                );
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettingsContext {
    pub project: PathBuf,
    pub tool: String,
    /// Empty key identifies the root; other keys identify the target and its parents.
    pub identities: BTreeMap<String, Option<String>>,
    pub original: Option<String>,
}

impl SettingsContext {
    pub fn target(&self) -> Result<&'static ConfigurableTool> {
        tool_settings::find(&self.tool).context("Unknown settings target")
    }
    pub fn validate(&self) -> Result<()> {
        let target = self.target()?;
        ensure!(
            self.project.is_absolute(),
            "Settings require an absolute captured project"
        );
        let mut expected = vec!["", target.target.relative()];
        if matches!(target.target, ConfigTarget::VsCodeSettings) {
            expected.push(".vscode");
        }
        ensure!(
            self.identities.len() == expected.len()
                && expected.iter().all(|p| self.identities.contains_key(*p)),
            "Invalid settings path identities"
        );
        ensure!(
            self.identities[""].as_ref().is_some_and(|v| !v.is_empty()),
            "Missing project identity"
        );
        ensure!(
            self.identities[target.target.relative()].is_some() == self.original.is_some(),
            "File identity does not match captured contents"
        );
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettingsReview {
    pub context: SettingsContext,
    pub draft: SettingsDraft,
    /// None for a no-op, including an absent file with no explicit edits.
    pub contents: Option<String>,
}

impl SettingsReview {
    pub fn validate(&self) -> Result<()> {
        self.context.validate()?;
        self.draft.validate(self.context.target()?)?;
        ensure!(
            self.contents.is_some() == self.draft.dirty(),
            "Settings output does not match pending edits"
        );
        if let Some(contents) = &self.contents {
            ensure!(
                self.context.original.as_ref() != Some(contents),
                "Settings review has no effective change"
            );
        }
        Ok(())
    }
    pub fn diff(&self) -> Result<Vec<crate::upgrade::diff::Row>> {
        self.validate()?;
        Ok(self.contents.as_ref().map_or_else(Vec::new, |text| {
            crate::upgrade::diff::rows(
                self.context.original.as_deref(),
                text,
                self.context.target().unwrap().target.relative(),
            )
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn absent_null_and_custom_values_remain_distinct_and_revert_exactly() {
        let tool = tool_settings::find("stylua-vscode").unwrap();
        let mut draft = SettingsDraft::new(vec![Some(Value::Null), None]);
        draft.validate(tool).unwrap();
        assert_eq!(draft.value(0), Some(&Value::Null));
        assert_eq!(draft.value(1), None);
        draft.set(0, json!(true));
        draft.set(1, json!(false));
        draft.validate(tool).unwrap();
        draft.revert(0);
        draft.revert(1);
        assert!(!draft.dirty());
        assert_eq!(draft.value(0), Some(&Value::Null));
        assert_eq!(draft.value(1), None);
    }

    #[test]
    fn exact_integer_and_choice_validation_rejects_wrong_json_types() {
        let integer = SettingKind::Integer { default: 1 };
        for value in [json!(i64::MIN), json!(i64::MAX), json!(0)] {
            assert!(integer.accepts(&value));
        }
        for value in [
            json!(u64::MAX),
            json!(1.0),
            json!("12"),
            json!([1]),
            Value::Null,
        ] {
            assert!(!integer.accepts(&value));
        }
        for tool in tool_settings::CONFIGURABLE_TOOLS {
            let mut draft = SettingsDraft::new(vec![None; tool.settings.len()]);
            for (i, setting) in tool.settings.iter().enumerate() {
                draft.set(i, setting.kind.default_value());
            }
            draft.validate(tool).unwrap();
            draft.changes[0] = Some(json!({"unknown":true}));
            assert!(draft.validate(tool).is_err());
        }
    }

    #[test]
    fn drafts_round_trip_and_preserve_unfamiliar_values_without_staging_them() {
        let tool = tool_settings::find("stylua-vscode").unwrap();
        let draft = SettingsDraft::new(vec![
            Some(json!(["custom", 7])),
            Some(json!({"future": true})),
        ]);
        draft.validate(tool).unwrap();
        assert!(!draft.dirty());
        let text = serde_json::to_string(&draft).unwrap();
        assert_eq!(serde_json::from_str::<SettingsDraft>(&text).unwrap(), draft);
    }
}
