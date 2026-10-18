//! Interface-independent Upgrade review data. Private desktop/CLI handoff;
//! filesystem planning and execution remain in the CLI.
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Component, Path, PathBuf},
};

pub mod diff;
pub mod protocol;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpgradeFile {
    pub relative: String,
    pub contents: String,
    pub reason: String,
    pub creating: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkippedFile {
    pub relative: String,
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpgradeReview {
    pub project: PathBuf,
    pub identity: String,
    pub originals: BTreeMap<String, Option<String>>,
    pub files: Vec<UpgradeFile>,
    pub skipped: Vec<SkippedFile>,
}

impl UpgradeReview {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.project.is_absolute() && !self.identity.is_empty(),
            "Upgrade review has no captured project identity"
        );
        let mut paths = BTreeSet::new();
        for relative in self
            .originals
            .keys()
            .chain(self.files.iter().map(|f| &f.relative))
            .chain(self.skipped.iter().map(|f| &f.relative))
        {
            ensure!(
                !relative.is_empty()
                    && !relative.contains('\\')
                    && Path::new(relative)
                        .components()
                        .all(|c| matches!(c, Component::Normal(_))),
                "Unsafe Upgrade path: {relative}"
            );
        }
        for file in &self.files {
            ensure!(paths.insert(&file.relative), "Duplicate Upgrade target");
            let original = self
                .originals
                .get(&file.relative)
                .ok_or_else(|| anyhow::anyhow!("Upgrade target has no captured source"))?;
            ensure!(
                file.creating == original.is_none() && original.as_deref() != Some(&file.contents),
                "Upgrade target does not match its captured source"
            );
        }
        for file in &self.skipped {
            ensure!(
                paths.insert(&file.relative) && self.originals.contains_key(&file.relative),
                "Invalid skipped Upgrade target"
            );
        }
        Ok(())
    }

    pub fn diff(&self, index: usize) -> Option<Vec<diff::Row>> {
        let file = self.files.get(index)?;
        Some(diff::rows(
            self.originals.get(&file.relative)?.as_deref(),
            &file.contents,
            &file.relative,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn review_round_trip_preserves_inputs_and_rejects_unsafe_targets() {
        let mut review = UpgradeReview {
            project: std::env::temp_dir(),
            identity: "captured".into(),
            originals: [(".luaurc".into(), None)].into(),
            files: vec![UpgradeFile {
                relative: ".luaurc".into(),
                contents: "{}\n".into(),
                reason: "strict mode".into(),
                creating: true,
            }],
            skipped: vec![],
        };
        review.validate().unwrap();
        assert_eq!(
            serde_json::from_str::<UpgradeReview>(&serde_json::to_string(&review).unwrap())
                .unwrap(),
            review
        );
        assert_eq!(review.diff(0).unwrap().last().unwrap().new_line, Some(1));
        review.files[0].relative = "../outside".into();
        assert!(review.validate().is_err());
    }
}
