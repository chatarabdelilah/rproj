use super::*;
use rproj_core::{
    creation::{CreationEnvironment, PackageWorkflow},
    saved_setups::{protocol::*, *},
    upgrade::diff::Row,
};

mod controller;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod ui_tests;
mod view;

#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
enum Step {
    #[default]
    Review,
    Dependencies,
    Capabilities,
    Files,
}
#[derive(Clone)]
struct Editing {
    loaded: LoadedSetup,
    draft: SavedSetupDraft,
}
pub(super) struct Prepared {
    review: ManagementReview,
    rows: Arc<Vec<Row>>,
}
#[derive(Clone)]
enum Task {
    Browse,
    Load(String, LoadPurpose),
    Prepare(CapturedSetup, SetupAction),
    Manage(Box<ManagementReview>),
    Reuse(CapturedSetup),
}
#[derive(Clone, Copy)]
enum LoadPurpose {
    Browse,
    Reload { replace_draft: bool },
}
impl Task {
    fn source(&self) -> Option<&CapturedSetup> {
        match self {
            Self::Prepare(s, _) | Self::Reuse(s) => Some(s),
            Self::Manage(r) => Some(&r.source),
            _ => None,
        }
    }
    fn operation(&self) -> Operation {
        match self {
            Self::Browse | Self::Load(..) => Operation::LoadSaved,
            Self::Prepare(..) => Operation::PrepareSaved,
            Self::Manage(_) => Operation::ManageSaved,
            Self::Reuse(_) => Operation::ReuseSaved,
        }
    }
}
pub(super) enum Response {
    Browse(SetupListing),
    Loaded(LoadedSetup),
    Prepared(Prepared),
    Managed(ManagementOutcome),
    Reused(ReusedSetup),
}
pub(super) struct Failure {
    message: String,
    teardown: bool,
    unconfirmed: bool,
}
impl From<anyhow::Error> for Failure {
    fn from(e: anyhow::Error) -> Self {
        Self {
            teardown: rproj_core::operations::owned_teardown_unconfirmed(&e),
            unconfirmed: false,
            message: format!("{e:#}"),
        }
    }
}
#[derive(Default)]
pub(super) struct SavedSession {
    pub(super) visible: bool,
    pub(super) support: Option<Result<(), String>>,
    listing: Option<SetupListing>,
    selected: Option<LoadedSetup>,
    draft: Option<Editing>,
    editing: bool,
    step: Step,
    prepared: Option<Prepared>,
    revision: u64,
    pending: Option<Task>,
    error: Option<String>,
    result: Option<ManagementOutcome>,
    reload_sources: std::collections::BTreeSet<PathBuf>,
    dialog: bool,
    close_pending: bool,
    last_confirmed: Option<(u64, bool)>,
    pub(super) teardown_uncertain: bool,
    naming: Option<bool>, // true Rename, false Duplicate
    show_diff: bool,
    selected_package: Option<String>,
}
fn path(source: &CapturedSetup) -> PathBuf {
    source.root.join(format!("{}.toml", source.name))
}
impl SavedSession {
    fn dirty(&self) -> bool {
        self.draft.as_ref().is_some_and(|d| d.draft.dirty())
    }
    fn name(&self) -> String {
        self.draft
            .as_ref()
            .map(|d| d.loaded.source.name.clone())
            .unwrap_or_else(|| "saved setup".into())
    }
    fn editable(&self) -> bool {
        self.pending.is_none()
            && !self.teardown_uncertain
            && self
                .draft
                .as_ref()
                .is_some_and(|d| !self.reload_sources.contains(&path(&d.loaded.source)))
    }
    fn changed(&mut self) {
        self.revision = self.revision.wrapping_add(1);
        self.prepared = None;
        self.error = None;
        self.result = None;
    }
    fn edit(&mut self, change: impl FnOnce(&mut SavedSetupDraft) -> anyhow::Result<()>) {
        if !self.editable() {
            return;
        }
        let mut draft = self.draft.as_ref().unwrap().draft.clone();
        match change(&mut draft) {
            Ok(()) => {
                self.draft.as_mut().unwrap().draft = draft;
                self.changed();
            }
            Err(e) => self.error = Some(format!("{e:#}")),
        }
    }
    fn begin_edit(&mut self, loaded: LoadedSetup) -> anyhow::Result<()> {
        anyhow::ensure!(
            loaded.edit_problem.is_none(),
            "{}",
            loaded
                .edit_problem
                .as_deref()
                .unwrap_or("Editing unavailable")
        );
        let draft = SavedSetupDraft::new(
            loaded
                .graph
                .clone()
                .ok_or_else(|| anyhow::anyhow!("No readable composition"))?,
        )?;
        self.draft = Some(Editing { loaded, draft });
        self.editing = true;
        self.step = Step::Review;
        self.changed();
        Ok(())
    }
    fn add_companions(&mut self) {
        self.edit(|draft| {
            let mut choices = draft.graph.package_set();
            rproj_core::creation::add_companions(&mut choices);
            let additions: Vec<_> = choices
                .into_iter()
                .filter(|k| !draft.graph.packages.contains(k))
                .collect();
            draft.graph.packages.extend(additions);
            Ok(())
        });
    }
    fn current_prepared(&self) -> Option<&Prepared> {
        let prepared = self.prepared.as_ref()?;
        match &prepared.review.action {
            SetupAction::Save(graph) => {
                let draft = self.draft.as_ref()?;
                (self.editing
                    && draft.loaded.source == prepared.review.source
                    && &draft.draft.graph == graph)
                    .then_some(prepared)
            }
            _ => (!self.editing).then_some(prepared),
        }
    }
    fn ready(&self) -> Option<&ManagementReview> {
        if self.pending.is_some() || self.teardown_uncertain {
            return None;
        }
        let review = &self.current_prepared()?.review;
        if self.reload_sources.contains(&path(&review.source)) {
            return None;
        }
        if let SetupAction::Save(graph) = &review.action {
            let draft = self.draft.as_ref()?;
            if !self.editing
                || draft.loaded.source != review.source
                || &draft.draft.graph != graph
                || review.document.as_ref()?.replacement.is_none()
            {
                return None;
            }
        }
        Some(review)
    }
    pub(super) fn row_count(&self) -> usize {
        self.current_prepared().map_or(0, |p| p.rows.len())
    }
    fn environment(&self) -> CreationEnvironment {
        self.listing
            .as_ref()
            .and_then(|l| l.environment.clone())
            .unwrap_or(CreationEnvironment {
                projects_root: PathBuf::new(),
                apps: vec![],
                extensions: vec![],
            })
    }
}
