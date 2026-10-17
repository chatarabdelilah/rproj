use super::*;
use rproj_core::creation::{CompositionDraft, CompositionPlan, composition_protocol::*};

mod controller;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod ui_tests;
mod view;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub(super) enum Step {
    #[default]
    Dependencies,
    Capabilities,
    Review,
}
impl Step {
    pub(super) const ALL: [Self; 3] = [Self::Dependencies, Self::Capabilities, Self::Review];
    pub(super) fn title(self) -> &'static str {
        match self {
            Self::Dependencies => "Dependencies",
            Self::Capabilities => "Capabilities",
            Self::Review => "Review",
        }
    }
    pub(super) fn index(self) -> usize {
        Self::ALL.iter().position(|s| *s == self).unwrap()
    }
}

#[derive(Default)]
pub(super) struct CompositionSession {
    pub(super) visible: bool,
    pub(super) project: Option<PathBuf>,
    pub(super) context: Option<CompositionContext>,
    pub(super) snapshot: Option<ProjectSnapshot>,
    pub(super) draft: Option<CompositionDraft>,
    pub(super) step: Step,
    pub(super) revision: u64,
    pub(super) context_id: u64,
    pub(super) loading: bool,
    pub(super) dirty: bool,
    pub(super) error: Option<String>,
    pub(super) unavailable: bool,
    pub(super) prepared: Option<(u64, CompositionPlan)>,
    pub(super) result: Option<Result<CompositionOutcome, String>>,
    pub(super) selected_package: Option<String>,
    pub(super) dialog_open: bool,
    pub(super) close_pending: bool,
    pub(super) teardown_uncertain: bool,
}
impl CompositionSession {
    pub(super) fn changed(&mut self) {
        self.revision = self.revision.wrapping_add(1);
        self.dirty = self
            .draft
            .as_ref()
            .is_some_and(|draft| draft.original != draft.graph);
        self.prepared = None;
        self.error = None;
    }
    pub(super) fn begin_load(&mut self, project: PathBuf) -> u64 {
        let id = self.context_id.wrapping_add(1);
        let revision = self.revision.wrapping_add(1);
        *self = Self {
            visible: true,
            project: Some(project),
            loading: true,
            context_id: id,
            revision,
            ..Default::default()
        };
        id
    }
    pub(super) fn accept_context(
        &mut self,
        id: u64,
        result: Result<(CompositionContext, ProjectSnapshot), String>,
    ) {
        if id != self.context_id {
            return;
        }
        self.loading = false;
        match result {
            Ok((context, snapshot)) => {
                self.project = Some(context.project.clone());
                self.draft = context.graph.clone().map(CompositionDraft::new);
                self.error = context.problem.clone();
                self.context = Some(context);
                self.snapshot = Some(snapshot);
                self.unavailable = false;
            }
            Err(error) => {
                self.unavailable = error.contains("Composition editing is unavailable")
                    || error.contains("Unsupported composition worker");
                self.error = Some(error);
            }
        }
    }
    pub(super) fn ready(&self) -> Result<(&CompositionContext, &CompositionDraft), String> {
        if self.loading {
            return Err("Reading captured project…".into());
        }
        let context = self
            .context
            .as_ref()
            .ok_or("Composition context is unavailable; Reload to recheck the bundled CLI")?;
        if let Some(problem) = &context.problem {
            return Err(problem.clone());
        }
        let draft = self
            .draft
            .as_ref()
            .ok_or("A recorded composition is required")?;
        draft.validate().map_err(|e| e.to_string())?;
        Ok((context, draft))
    }
    pub(super) fn request(&self) -> Result<PrepareComposition, String> {
        let (context, draft) = self.ready()?;
        Ok(PrepareComposition {
            context: context.clone(),
            draft: draft.clone(),
        })
    }
    pub(super) fn fresh_plan(&self) -> Option<&CompositionPlan> {
        let (revision, plan) = self.prepared.as_ref()?;
        let (_, draft) = self.ready().ok()?;
        (self.result.is_none()
            && self.revision == *revision
            && plan.draft == *draft
            && Some(&plan.project) == self.project.as_ref())
        .then_some(plan)
    }
    pub(super) fn watch_conflict(&self, watch: Option<&WatchStatus>) -> bool {
        watch.is_some_and(|watch| {
            self.project.is_some()
                && watch.project == self.project
                && matches!(
                    watch.state,
                    rproj_core::operations::WatchState::Preparing
                        | rproj_core::operations::WatchState::Watching
                        | rproj_core::operations::WatchState::Stopping
                        | rproj_core::operations::WatchState::Unresponsive
                        | rproj_core::operations::WatchState::Unknown
                )
        })
    }
    pub(super) fn reset(&mut self) {
        *self = Self {
            context_id: self.context_id.wrapping_add(1),
            revision: self.revision.wrapping_add(1),
            ..Default::default()
        };
    }
}

fn outcome_label(status: CompositionStatus) -> &'static str {
    match status {
        CompositionStatus::Succeeded => "Changes applied",
        CompositionStatus::Failed => "Changes failed",
        CompositionStatus::Cancelled => "Changes cancelled",
        CompositionStatus::Uncertain => "Outcome unconfirmed",
    }
}
fn writes_label(writes: WriteState) -> &'static str {
    match writes {
        WriteState::NotStarted => "No composition writes started",
        WriteState::MayHaveBegun => "Changes may have started; inspect the project",
        WriteState::Completed => "Required steps completed and the composition record was saved",
        WriteState::Unknown => "File changes could not be verified",
    }
}

pub(super) struct CompositionInputs {
    pub(super) focus: FocusHandle,
    pub(super) query: Entity<InputState>,
}
impl CompositionInputs {
    pub(super) fn new(window: &mut Window, cx: &mut Context<Workbench>) -> (Self, Subscription) {
        let query = cx.new(|cx| InputState::new(window, cx).placeholder("Search packages"));
        let subscription = cx.subscribe(&query, |_, _, _: &InputEvent, cx| cx.notify());
        (
            Self {
                focus: cx.focus_handle(),
                query,
            },
            subscription,
        )
    }
}
