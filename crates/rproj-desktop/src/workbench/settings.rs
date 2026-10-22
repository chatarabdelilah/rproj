use super::*;
use rproj_core::{
    catalog::tool_settings::{self, SettingKind},
    tool_configuration::{SettingsReview, protocol::*},
    upgrade::diff::Row,
};
use std::collections::BTreeSet;

mod controller;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod ui_tests;
mod view;

pub(super) struct Prepared {
    review: SettingsReview,
    rows: Arc<Vec<Row>>,
}
pub(super) struct ReadFailure {
    message: String,
    teardown: bool,
}
impl From<anyhow::Error> for ReadFailure {
    fn from(error: anyhow::Error) -> Self {
        Self {
            teardown: rproj_core::operations::owned_teardown_unconfirmed(&error),
            message: format!("{error:#}"),
        }
    }
}

#[derive(Default)]
pub(super) struct SettingsSession {
    pub(super) visible: bool,
    pub(super) project: Option<PathBuf>,
    tool: String,
    loaded: Option<SettingsLoaded>,
    prepared: Option<Prepared>,
    revision: u64,
    reviewing: bool,
    loading: bool,
    error: Option<String>,
    pub(super) support: Option<Result<(), String>>,
    consent: BTreeSet<usize>,
    result: Option<Result<SettingsOutcome, String>>,
    reload_required: bool,
    dialog_open: bool,
    close_pending: bool,
    pub(super) teardown_uncertain: bool,
}
impl SettingsSession {
    pub(super) fn name(&self) -> String {
        self.project
            .as_ref()
            .and_then(|p| p.file_name())
            .unwrap_or_default()
            .to_string_lossy()
            .into()
    }
    fn dirty(&self) -> bool {
        self.loaded.as_ref().is_some_and(|l| l.draft.dirty())
    }
    fn changed(&mut self) {
        self.revision = self.revision.wrapping_add(1);
        self.prepared = None;
        self.reviewing = false;
        self.error = None;
        self.result = None;
    }
    fn editable(&self, index: usize) -> bool {
        let Some(loaded) = &self.loaded else {
            return false;
        };
        let Ok(tool) = loaded.context.target() else {
            return false;
        };
        let Some(setting) = tool.settings.get(index) else {
            return false;
        };
        !self.loading
            && !self.reload_required
            && !self.teardown_uncertain
            && !loaded.blocked.contains_key(&index)
            && (loaded.draft.current[index]
                .as_ref()
                .is_none_or(|v| setting.kind.accepts(v))
                || self.consent.contains(&index))
    }
    fn set(&mut self, index: usize, value: serde_json::Value) {
        if !self.editable(index) {
            return;
        }
        let loaded = self.loaded.as_mut().unwrap();
        if !loaded.context.target().unwrap().settings[index]
            .kind
            .accepts(&value)
        {
            return;
        }
        loaded.draft.set(index, value);
        self.changed();
    }
    fn ready(&self) -> Option<&SettingsReview> {
        if self.loading || self.reload_required || self.teardown_uncertain || self.error.is_some() {
            return None;
        }
        let review = &self.prepared.as_ref()?.review;
        let loaded = self.loaded.as_ref()?;
        (self.reviewing
            && review.contents.is_some()
            && review.context == loaded.context
            && review.draft == loaded.draft
            && Some(&review.context.project) == self.project.as_ref()
            && review.context.tool == self.tool)
            .then_some(review)
    }
    fn watch_conflict(&self, watch: Option<&WatchStatus>) -> bool {
        watch.is_some_and(|w| {
            self.project.is_some()
                && w.project == self.project
                && !matches!(
                    w.state,
                    rproj_core::operations::WatchState::Stopped
                        | rproj_core::operations::WatchState::Failed
                )
        })
    }
    pub(super) fn row_count(&self) -> usize {
        self.prepared.as_ref().map_or(0, |p| p.rows.len())
    }
}
