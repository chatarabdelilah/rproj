mod controller;
#[cfg(test)]
mod tests;
mod view;

use super::*;
use rproj_core::creation::{CreationDraft, protocol::*};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Step {
    Project,
    Dependencies,
    Capabilities,
    Files,
    Review,
}
impl Step {
    pub(super) const ALL: [Self; 5] = [
        Self::Project,
        Self::Dependencies,
        Self::Capabilities,
        Self::Files,
        Self::Review,
    ];
    pub(super) fn title(self) -> &'static str {
        match self {
            Self::Project => "Project",
            Self::Dependencies => "Dependencies",
            Self::Capabilities => "Capabilities",
            Self::Files => "Files",
            Self::Review => "Review",
        }
    }
    pub(super) fn index(self) -> usize {
        Self::ALL.iter().position(|s| *s == self).unwrap()
    }
}

pub(super) struct CreationSession {
    pub(super) visible: bool,
    pub(super) draft: Option<CreationDraft>,
    pub(super) step: Step,
    pub(super) revision: u64,
    pub(super) dirty: bool,
    pub(super) context: Option<CreationContext>,
    pub(super) context_id: u64,
    pub(super) loading: bool,
    pub(super) error: Option<String>,
    pub(super) selection_problem: Option<String>,
    pub(super) selected_setup: Option<String>,
    pub(super) setup_source: Option<SourceSnapshot>,
    pub(super) prepared: Option<(u64, PreparedCreation)>,
    pub(super) result: Option<Result<CreationOutcome, String>>,
    pub(super) category: rproj_core::catalog::wally_packages::Category,
    pub(super) selected_package: Option<String>,
    pub(super) close_pending: bool,
    pub(super) close_dialog: bool,
    pub(super) teardown_uncertain: bool,
}
impl Default for CreationSession {
    fn default() -> Self {
        Self {
            visible: false,
            draft: None,
            step: Step::Project,
            revision: 0,
            dirty: false,
            context: None,
            context_id: 0,
            loading: false,
            error: None,
            selection_problem: None,
            selected_setup: None,
            setup_source: None,
            prepared: None,
            result: None,
            category: rproj_core::catalog::wally_packages::Category::Ui,
            selected_package: None,
            close_pending: false,
            close_dialog: false,
            teardown_uncertain: false,
        }
    }
}
impl CreationSession {
    pub(super) fn changed(&mut self) {
        self.revision = self.revision.wrapping_add(1);
        self.dirty = true;
        self.prepared = None;
        self.error = None;
    }
    pub(super) fn destination(&self) -> Option<PathBuf> {
        Some(
            self.context
                .as_ref()?
                .environment
                .projects_root
                .join(&self.draft.as_ref()?.name),
        )
    }
    pub(super) fn project_ready(&self) -> Result<(), String> {
        let context = self
            .context
            .as_ref()
            .ok_or("Creation context is unavailable. Recheck the bundled CLI.")?;
        if self.loading {
            return Err("Rechecking creation context…".into());
        }
        if !context.machine_ready {
            return Err(
                "Machine setup is missing. Run rproj setup in a terminal, then Recheck.".into(),
            );
        }
        if let Some(error) = &context.template_problem {
            return Err(format!("Template: {error}"));
        }
        if let Some(error) = &context.setups_problem {
            return Err(format!("Saved setups: {error}"));
        }
        let draft = self.draft.as_ref().ok_or("Start a new draft.")?;
        rproj_core::creation::validate_name(&draft.name).map_err(str::to_owned)?;
        Ok(())
    }
    pub(super) fn request(&self) -> Result<PrepareCreation, String> {
        self.project_ready()?;
        if let Some(problem) = &self.selection_problem {
            return Err(problem.clone());
        }
        let context = self.context.as_ref().expect("project readiness");
        let draft = self.draft.as_ref().expect("project readiness");
        draft.validate().map_err(|e| e.to_string())?;
        let sources = context
            .sources(self.selected_setup.as_deref())
            .map_err(|e| e.to_string())?;
        if let Some((_, source)) = &sources.selected_setup
            && self.setup_source.as_ref() != Some(source)
        {
            return Err(
                "The selected saved setup changed. Reload it explicitly before review.".into(),
            );
        }
        Ok(PrepareCreation {
            draft: draft.clone(),
            sources,
        })
    }
    pub(super) fn fresh_plan(&self) -> Option<&PreparedCreation> {
        let (revision, prepared) = self.prepared.as_ref()?;
        let request = self.request().ok()?;
        (*revision == self.revision
            && request.draft == prepared.plan.draft
            && request.sources == prepared.sources)
            .then_some(prepared)
    }
    pub(super) fn load_setup(&mut self, name: &str) -> Result<(), String> {
        let result = self.apply_setup(name);
        self.selection_problem = result.as_ref().err().cloned();
        self.prepared = None;
        result
    }
    pub(super) fn select_mode(&mut self, mode: &str) -> Result<(), String> {
        use rproj_core::{
            catalog::wally_packages::{self, Category},
            creation::{add_companions, apply_derived_packages, capability_owned, graph::Node},
        };
        let mut draft = self.draft.clone().ok_or("Start a draft first.")?;
        if mode == "guided" {
            // Validate before filtering: an unfamiliar selection must never be
            // silently removed by a conversion into Guided's primary choices.
            let mut validation = draft.clone();
            validation.name = "Validation".into();
            validation.save_setup = None;
            validation.validate().map_err(|e| e.to_string())?;
            let mut choices: std::collections::BTreeSet<String> = draft
                .graph
                .packages
                .iter()
                .filter(|key| {
                    wally_packages::find(key).is_some_and(|p| p.primary_choice)
                        && !capability_owned(key)
                })
                .cloned()
                .collect();
            for category in Category::ALL.into_iter().filter(|c| !c.allows_multiple()) {
                if choices
                    .iter()
                    .filter(|key| wally_packages::find(key).is_some_and(|p| p.category == category))
                    .count()
                    > 1
                {
                    return Err(format!(
                        "Guided allows one primary choice in {}. Remove the extra choices in Expert, then select Guided again.",
                        category.label()
                    ));
                }
            }
            add_companions(&mut choices);
            let previous = draft.graph.packages.clone();
            draft.graph.packages = choices.into_iter().collect();
            apply_derived_packages(&mut draft.graph);
            if draft.graph.packages != previous {
                draft.graph.invalidate(Node::Packages);
            }
        } else if mode != "expert" {
            return Err("Unsupported creation mode.".into());
        }
        draft.graph.mode = mode.into();
        self.draft = Some(draft);
        self.selected_setup = None;
        self.setup_source = None;
        self.selection_problem = None;
        self.changed();
        Ok(())
    }
    fn apply_setup(&mut self, name: &str) -> Result<(), String> {
        let saved = self
            .context
            .as_ref()
            .and_then(|c| c.setups.iter().find(|s| s.name == name))
            .ok_or("Saved setup is unavailable. Recheck.")?;
        if let Some(problem) = &saved.problem {
            return Err(format!("{name}: {problem}"));
        }
        let graph = saved
            .graph
            .clone()
            .ok_or("Saved composition cannot be read.")?;
        let mut candidate = self.draft.clone().ok_or("Start a draft first.")?;
        candidate.graph = graph;
        // Validate composition independently from the still editable project name.
        let mut validation = candidate.clone();
        validation.name = "Validation".into();
        validation.save_setup = None;
        validation.validate().map_err(|e| format!("{name}: {e}"))?;
        self.setup_source = Some(saved.source.clone());
        self.selected_setup = Some(name.into());
        self.draft = Some(candidate);
        self.changed();
        Ok(())
    }
}

pub(super) struct CreationInputs {
    pub(super) focus: FocusHandle,
    pub(super) name: Entity<InputState>,
    pub(super) setup: Entity<InputState>,
    pub(super) query: Entity<InputState>,
}
impl CreationInputs {
    pub(super) fn new(
        window: &mut Window,
        cx: &mut Context<Workbench>,
    ) -> (Self, Vec<Subscription>) {
        let name = cx.new(|cx| InputState::new(window, cx).placeholder("Project name"));
        let setup = cx.new(|cx| InputState::new(window, cx).placeholder("Optional new setup name"));
        let query = cx.new(|cx| InputState::new(window, cx).placeholder("Search packages"));
        let subscriptions = vec![
            cx.subscribe(&name, |this, input, event, cx| {
                if matches!(event, InputEvent::Change)
                    && !this.creation_running()
                    && this.creation.result.is_none()
                {
                    let value = input.read(cx).value().to_string();
                    if let Some(draft) = this.creation.draft.as_mut()
                        && draft.name != value
                    {
                        draft.name = value;
                        this.creation.changed();
                    }
                    cx.notify();
                }
            }),
            cx.subscribe(&setup, |this, input, event, cx| {
                if matches!(event, InputEvent::Change)
                    && !this.creation_running()
                    && this.creation.result.is_none()
                {
                    let value = input.read(cx).value().to_string();
                    let value = (!value.is_empty()).then_some(value);
                    if let Some(draft) = this.creation.draft.as_mut()
                        && draft.save_setup != value
                    {
                        draft.save_setup = value;
                        this.creation.changed();
                    }
                    cx.notify();
                }
            }),
            cx.subscribe(&query, |_, _, _: &InputEvent, cx| cx.notify()),
        ];
        (
            Self {
                focus: cx.focus_handle(),
                name,
                setup,
                query,
            },
            subscriptions,
        )
    }
}
