use super::*;
use rproj_core::{
    catalog::tool_catalog::{self, ToolKind},
    machine_setup::{
        Category, Detection, Persistence, Selection, SetupContext, SetupReview, Status, protocol::*,
    },
    operations::{SetupEvent, SetupStop},
};

mod controller;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod ui_tests;
mod view;

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum Step {
    #[default]
    Selections,
    Review,
    Results,
}
impl Step {
    fn title(self) -> &'static str {
        match self {
            Self::Selections => "Selections",
            Self::Review => "Review",
            Self::Results => "Results",
        }
    }
}

pub(super) struct Failure {
    message: String,
    teardown: bool,
}
impl From<anyhow::Error> for Failure {
    fn from(error: anyhow::Error) -> Self {
        Self {
            teardown: rproj_core::operations::owned_teardown_unconfirmed(&error),
            message: format!("{error:#}"),
        }
    }
}

pub(super) struct SetupSession {
    pub(super) visible: bool,
    pub(super) support: Option<Result<(), String>>,
    context: Option<SetupContext>,
    selection: Option<Selection>,
    retry_selection: Option<Selection>,
    prepared: Option<SetupReview>,
    result: Option<SetupReport>,
    progress: Option<Box<SetupReport>>,
    revision: u64,
    step: Step,
    category: Category,
    selected: Option<String>,
    loading: bool,
    error: Option<String>,
    reload_required: bool,
    dialog_open: bool,
    close_pending: bool,
    stop: Option<SetupStop>,
    pub(super) teardown_uncertain: bool,
}
impl Default for SetupSession {
    fn default() -> Self {
        Self {
            visible: false,
            support: None,
            context: None,
            selection: None,
            retry_selection: None,
            prepared: None,
            result: None,
            progress: None,
            revision: 0,
            step: Step::Selections,
            category: Category::Apps,
            selected: None,
            loading: false,
            error: None,
            reload_required: false,
            dialog_open: false,
            close_pending: false,
            stop: None,
            teardown_uncertain: false,
        }
    }
}
impl SetupSession {
    fn dirty(&self) -> bool {
        self.selection.as_ref().is_some_and(|selection| {
            self.context
                .as_ref()
                .is_none_or(|context| context.saved != *selection)
        })
    }
    fn toggle(&mut self, category: Category, key: &str) {
        if self.loading || self.reload_required || self.teardown_uncertain || self.result.is_some()
        {
            return;
        }
        let Some(selection) = &mut self.selection else {
            return;
        };
        if !selection.active(category) || !category.entries().iter().any(|e| e.key == key) {
            return;
        }
        let mut keys = selection.keys(category).clone();
        if !keys.remove(key) {
            keys.insert(key.into());
        }
        selection.replace_category(category, &keys);
        self.revision = self.revision.wrapping_add(1);
        self.prepared = None;
        self.error = None;
        self.step = Step::Selections;
    }
    fn ready(&self) -> Option<&SetupReview> {
        if self.loading
            || !matches!(self.support, Some(Ok(())))
            || self.reload_required
            || self.teardown_uncertain
            || self.error.is_some()
            || self.result.is_some()
        {
            return None;
        }
        let review = self.prepared.as_ref()?;
        (self.context.as_ref() == Some(&review.context)
            && self.selection.as_ref() == Some(&review.selection))
        .then_some(review)
    }
    fn watch_conflict(watch: Option<&WatchStatus>) -> bool {
        watch.is_some_and(|w| {
            !matches!(
                w.state,
                rproj_core::operations::WatchState::Stopped
                    | rproj_core::operations::WatchState::Failed
            )
        })
    }
    fn accept_progress(&mut self, report: Box<SetupReport>) -> bool {
        if !self.loading
            || report.status.is_some()
            || report.machine.as_ref() != self.context.as_ref().map(|c| &c.config_path)
            || self
                .progress
                .as_ref()
                .is_some_and(|p| p.id != report.id || p.sequence >= report.sequence)
        {
            return false;
        }
        let failure = self.has_new_failure(&report);
        self.progress = Some(report);
        failure
    }
    fn has_new_failure(&self, report: &SetupReport) -> bool {
        report.statuses.iter().enumerate().any(|(i, s)| {
            *s == Status::Failed
                && self.progress.as_ref().and_then(|p| p.statuses.get(i)) != Some(&Status::Failed)
        })
    }
}
