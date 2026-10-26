use super::Operation;
use rproj_core::operations::{OperationOutcome, WatchStatus};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum RunStatus {
    Running,
    Cancelling,
    StoppingAfterItem,
    Stopped,
    Succeeded,
    Failed(String),
    Cancelled,
}

impl RunStatus {
    pub(super) fn label(&self) -> &str {
        match self {
            Self::Running => "Running",
            Self::Cancelling => "Cancellation requested…",
            Self::StoppingAfterItem => "Stopping after active item…",
            Self::Stopped => "Stopped",
            Self::Succeeded => "Succeeded",
            Self::Failed(message) => message,
            Self::Cancelled => "Cancelled",
        }
    }
}

pub(super) struct Run {
    pub(super) id: u64,
    pub(super) operation: Operation,
    pub(super) project: PathBuf,
    pub(super) status: RunStatus,
}
impl Run {
    pub(super) fn origin(&self) -> String {
        let owner = if self.operation.is_setup() {
            "Machine Setup".into()
        } else {
            self.project.display().to_string()
        };
        format!("{owner} · {}", super::operation_label(self.operation))
    }
}

#[derive(Default)]
pub(super) struct OperationPresentation {
    next_id: u64,
    pub(super) latest: Option<Run>,
    pub(super) expanded: bool,
}

impl OperationPresentation {
    pub(super) fn begin(&mut self, operation: Operation, project: PathBuf) -> u64 {
        self.next_id = self.next_id.wrapping_add(1);
        self.latest = Some(Run {
            id: self.next_id,
            operation,
            project,
            status: RunStatus::Running,
        });
        self.expanded = true;
        self.next_id
    }

    pub(super) fn accepts(&self, id: u64) -> bool {
        self.latest.as_ref().is_some_and(|run| {
            run.id == id
                && matches!(
                    run.status,
                    RunStatus::Running | RunStatus::Cancelling | RunStatus::StoppingAfterItem
                )
        })
    }

    pub(super) fn cancel(&mut self) {
        if let Some(run) = self.latest.as_mut()
            && run.operation.cancellable()
            && run.status == RunStatus::Running
        {
            run.status = RunStatus::Cancelling;
        }
    }
    pub(super) fn stop_setup(&mut self) {
        if let Some(run) = self.latest.as_mut()
            && run.operation == Operation::ApplySetup
            && run.status == RunStatus::Running
        {
            run.status = RunStatus::StoppingAfterItem;
        }
    }

    pub(super) fn finish(&mut self, id: u64, result: &Result<OperationOutcome, String>) -> bool {
        if !self.accepts(id) {
            return false;
        }
        let status = match result {
            Ok(OperationOutcome::Succeeded) => RunStatus::Succeeded,
            Ok(OperationOutcome::Cancelled)
                if self.latest.as_ref().unwrap().operation == Operation::ApplySetup =>
            {
                RunStatus::Stopped
            }
            Ok(OperationOutcome::Cancelled) => RunStatus::Cancelled,
            Ok(OperationOutcome::Failed { code: Some(code) }) => {
                RunStatus::Failed(format!("Failed (exit code {code})"))
            }
            Ok(OperationOutcome::Failed { code: None }) => {
                RunStatus::Failed("Failed (no exit code)".into())
            }
            Err(error) => RunStatus::Failed(format!("Could not complete operation: {error}")),
        };
        if matches!(status, RunStatus::Failed(_)) {
            self.expanded = true;
        }
        self.latest.as_mut().unwrap().status = status;
        true
    }

    pub(super) fn summary(&self) -> String {
        self.latest
            .as_ref()
            .map(|run| format!("{} · {}", run.origin(), run.status.label()))
            .unwrap_or_else(|| "No operations yet".into())
    }

    pub(super) fn compact_summary(&self) -> String {
        self.latest
            .as_ref()
            .map(|run| {
                let project = if run.operation.is_setup() {
                    std::borrow::Cow::Borrowed("Machine Setup")
                } else {
                    run.project
                        .file_name()
                        .unwrap_or(run.project.as_os_str())
                        .to_string_lossy()
                };
                let status = match &run.status {
                    RunStatus::Failed(_) => "Failed",
                    other => other.label(),
                };
                format!(
                    "{project} · {} · {status}",
                    super::operation_label(run.operation)
                )
            })
            .unwrap_or_else(|| "No operations yet".into())
    }
}

#[derive(Clone)]
pub(super) struct WatchSwitch {
    pub(super) from: PathBuf,
    pub(super) to: PathBuf,
    pub(super) generation: u64,
}

impl WatchSwitch {
    pub(super) fn matches(
        &self,
        project: Option<&Path>,
        watch: Option<&WatchStatus>,
        generation: u64,
    ) -> bool {
        self.generation == generation
            && project == Some(self.to.as_path())
            && watch.is_some_and(|watch| {
                watch.supported
                    && watch.project.as_deref() == Some(self.from.as_path())
                    && matches!(
                        watch.state,
                        rproj_core::operations::WatchState::Watching
                            | rproj_core::operations::WatchState::Preparing
                    )
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rproj_core::operations::WatchState;

    #[test]
    fn collapse_survives_success_but_failure_reopens_and_stale_results_are_rejected() {
        let mut state = OperationPresentation::default();
        assert!(!state.expanded);
        assert_eq!(state.summary(), "No operations yet");
        let first = state.begin(Operation::Check, PathBuf::from("original-project"));
        assert!(state.expanded);
        state.expanded = false;
        assert!(state.finish(first, &Ok(OperationOutcome::Succeeded)));
        assert!(!state.expanded);
        assert!(state.summary().contains("original-project"));
        let second = state.begin(Operation::Test, PathBuf::from("other-project"));
        state.expanded = false;
        assert!(!state.finish(first, &Err("late failure".into())));
        assert!(!state.expanded);
        assert!(state.finish(second, &Err("runner failed".into())));
        assert!(state.expanded);
        assert!(!state.accepts(second));
        assert!(!state.finish(second, &Ok(OperationOutcome::Succeeded)));
        assert!(state.summary().contains("runner failed"));
    }

    #[test]
    fn cancellation_is_pending_until_worker_acknowledges() {
        let mut state = OperationPresentation::default();
        let id = state.begin(Operation::Check, PathBuf::from("project"));
        state.cancel();
        assert_eq!(state.latest.as_ref().unwrap().status, RunStatus::Cancelling);
        assert!(state.accepts(id));
        state.expanded = false;
        assert!(state.finish(id, &Ok(OperationOutcome::Cancelled)));
        assert_eq!(state.latest.as_ref().unwrap().status, RunStatus::Cancelled);
        assert!(!state.expanded);
        state.begin(Operation::StopWatch, PathBuf::from("project"));
        state.cancel();
        assert_eq!(state.latest.as_ref().unwrap().status, RunStatus::Running);
    }

    #[test]
    fn switch_confirmation_cannot_follow_navigation_or_changed_watch() {
        let from = PathBuf::from("from");
        let to = PathBuf::from("to");
        let confirmation = WatchSwitch {
            from: from.clone(),
            to: to.clone(),
            generation: 4,
        };
        let mut watch = WatchStatus {
            supported: true,
            project: Some(from),
            state: WatchState::Watching,
            message: String::new(),
        };
        assert!(confirmation.matches(Some(&to), Some(&watch), 4));
        assert!(!confirmation.matches(Some(Path::new("third")), Some(&watch), 4));
        assert!(!confirmation.matches(Some(&to), Some(&watch), 5));
        assert!(!confirmation.matches(None, Some(&watch), 4));
        watch.state = WatchState::Unresponsive;
        assert!(!confirmation.matches(Some(&to), Some(&watch), 4));
        watch.state = WatchState::Watching;
        watch.project = Some(PathBuf::from("third"));
        assert!(!confirmation.matches(Some(&to), Some(&watch), 4));
    }
}
