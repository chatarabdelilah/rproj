use super::*;
use rproj_core::{
    catalog::tool_catalog::{self, ToolEntry, ToolKind},
    project_tool_setup::{
        Plan, Step, StepState,
        protocol::{Completion, Status, WriteState},
    },
};

mod controller;
#[cfg(test)]
mod tests;
mod view;

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

#[derive(Default)]
pub(super) struct Session {
    pub(super) visible: bool,
    pub(super) project: Option<PathBuf>,
    pub(super) tool: String,
    pub(super) revision: u64,
    pub(super) plan: Option<Plan>,
    pub(super) result: Option<Completion>,
    pub(super) error: Option<String>,
    pub(super) loading: bool,
    pub(super) support: Option<Result<(), String>>,
    pub(super) dialog_open: bool,
    pub(super) close_pending: bool,
    pub(super) teardown_uncertain: bool,
    pub(super) completion_confirmed: bool,
}
impl Session {
    pub(super) fn name(&self) -> String {
        self.project
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|| "captured project".into())
    }
    fn ready(&self) -> Option<&Plan> {
        self.plan.as_ref().filter(|plan| {
            !self.loading
                && self.result.is_none()
                && self.error.is_none()
                && !self.teardown_uncertain
                && self.project.as_ref() == Some(&plan.project)
                && self.tool == plan.tool
        })
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
    fn choose(&mut self, tool: &str) {
        self.tool = tool.into();
        self.revision = self.revision.wrapping_add(1);
        self.plan = None;
        self.result = None;
        self.error = None;
        self.completion_confirmed = false;
    }
}

fn scope(tool: &ToolEntry) -> &'static str {
    if tool.key == "blender" {
        "Project starter scene · installed Blender required"
    } else if matches!(tool.kind, ToolKind::RokitTool { .. }) {
        "Project Rokit tool"
    } else {
        "Machine-wide / guidance only"
    }
}

fn choices(query: &str) -> Vec<&'static ToolEntry> {
    let query = query.trim().to_lowercase();
    let mut tools: Vec<_> = tool_catalog::all_setup_entries()
        .filter(|tool| {
            format!(
                "{} {} {} {}",
                tool.key,
                tool.description,
                tool.kind.label(),
                scope(tool)
            )
            .to_lowercase()
            .contains(&query)
        })
        .collect();
    tools.sort_by_key(|tool| tool.key);
    tools
}
