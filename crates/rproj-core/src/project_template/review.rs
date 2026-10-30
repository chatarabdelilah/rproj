//! Captured template state and deterministic, filesystem-independent review.
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::BTreeMap, path::PathBuf};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TemplateSource {
    pub path: PathBuf,
    /// Every parent, nearest first, including missing directories.
    pub parents: BTreeMap<PathBuf, Option<String>>,
    pub identity: Option<String>,
    pub original: Option<String>,
}

impl TemplateSource {
    pub fn validate(&self) -> Result<()> {
        ensure!(self.path.is_absolute(), "Template path must be absolute");
        ensure!(
            self.identity.is_some() == self.original.is_some(),
            "Missing template identity"
        );
        let expected: Vec<_> = self.path.ancestors().skip(1).collect();
        ensure!(
            expected.len() == self.parents.len()
                && expected.iter().all(|p| self.parents.contains_key(*p)),
            "Incomplete template parent identities"
        );
        ensure!(
            self.parents
                .values()
                .chain(std::iter::once(&self.identity))
                .flatten()
                .all(|id| !id.is_empty()),
            "Empty template identity"
        );
        Ok(())
    }

    pub fn value(&self) -> Result<Value> {
        match &self.original {
            Some(text) => serde_json::from_str(text).context(
                "Invalid template JSON; repair in Advanced JSON or restore the built-in template",
            ),
            None => Ok(super::builtin_project_template()),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TemplateAction {
    Save(Value),
    Reset,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TemplateReview {
    pub source: TemplateSource,
    pub action: TemplateAction,
    /// Save bytes only; Reset removes the captured override.
    pub replacement: Option<String>,
}

impl TemplateReview {
    pub fn prepare(source: TemplateSource, action: TemplateAction) -> Result<Self> {
        source.validate()?;
        let replacement = match &action {
            TemplateAction::Save(value) => {
                super::model::representable(value)?;
                super::validate_template_structure(value)?;
                if source.value().ok().as_ref() == Some(value) {
                    None
                } else {
                    Some(format!("{}\n", serde_json::to_string_pretty(value)?))
                }
            }
            TemplateAction::Reset => None,
        };
        Ok(Self {
            source,
            action,
            replacement,
        })
    }

    pub fn validate(&self) -> Result<()> {
        ensure!(
            Self::prepare(self.source.clone(), self.action.clone())? == *self,
            "Template review does not match its captured draft; review again"
        );
        Ok(())
    }

    pub fn changes(&self) -> bool {
        match self.action {
            TemplateAction::Save(_) => self.replacement.is_some(),
            TemplateAction::Reset => self.source.original.is_some(),
        }
    }

    pub fn diff(&self) -> Vec<crate::upgrade::diff::Row> {
        let old = self.source.original.as_deref().unwrap_or("");
        let new = match &self.action {
            TemplateAction::Save(_) => self.replacement.as_deref().unwrap_or(old),
            TemplateAction::Reset => "",
        };
        crate::upgrade::diff::rows(self.source.original.as_deref(), new, "default.project.json")
    }
}
