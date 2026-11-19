use super::*;
use rproj_core::test_execution::{
    Effect, TestContext,
    protocol::{Completion, Status},
};

mod controller;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod ui_tests;
mod view;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Mode {
    #[default]
    Literal,
    Json,
}

#[derive(Clone, Debug)]
struct Row {
    id: u64,
    text: String,
    mode: Mode,
    empty: bool,
    last_valid: String,
}
impl Row {
    fn edit(&mut self, text: String) {
        self.text = text;
        if let Ok(value) = self.value() {
            self.last_valid = value;
        }
    }
    fn value(&self) -> Result<String, String> {
        let value = match self.mode {
            Mode::Literal => {
                if self.text.contains(['\r', '\n', '\t']) {
                    return Err("Use JSON string mode for line breaks and tabs.".into());
                }
                self.text.clone()
            }
            Mode::Json => serde_json::from_str::<String>(&self.text)
                .map_err(|_| "Enter one JSON string, including its quotation marks.".to_owned())?,
        };
        if value.contains('\0') {
            return Err("NUL cannot be passed to a process.".into());
        }
        if value.is_empty() && !self.empty {
            return Err("Choose Pass empty argument, or remove this row.".into());
        }
        Ok(value)
    }
    fn switch_mode(&mut self, mode: Mode) -> Result<(), String> {
        if mode == self.mode {
            return Ok(());
        }
        // Presentation changes do not submit or change the actual argument.
        let value = match self.mode {
            Mode::Literal => self.text.clone(),
            Mode::Json => serde_json::from_str::<String>(&self.text)
                .map_err(|_| "Repair the JSON string before changing modes.".to_owned())?,
        };
        if mode == Mode::Literal && value.chars().any(char::is_control) {
            return Err("Keep JSON string mode for control characters.".into());
        }
        self.text = if mode == Mode::Json {
            serde_json::to_string(&value).unwrap()
        } else {
            value
        };
        self.mode = mode;
        Ok(())
    }
    fn paste(&mut self, text: &str, selection: std::ops::Range<usize>) -> Result<bool, String> {
        if !text.chars().any(char::is_control) {
            return Ok(false);
        }
        if self.mode == Mode::Json {
            return Err("Paste JSON escapes such as \\r and \\n, not raw control characters, in JSON string mode.".into());
        }
        if text.contains('\0') {
            return Err("NUL cannot be passed to a process.".into());
        }
        if !self.text.is_char_boundary(selection.start)
            || !self.text.is_char_boundary(selection.end)
            || selection.start > selection.end
        {
            return Err("Selection changed; try the paste again.".into());
        }
        let mut value = self.text.clone();
        value.replace_range(selection, text);
        self.text = serde_json::to_string(&value).unwrap();
        self.mode = Mode::Json;
        self.last_valid = value;
        Ok(true)
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

#[derive(Default)]
pub(super) struct Session {
    pub(super) visible: bool,
    pub(super) project: Option<PathBuf>,
    context: Option<TestContext>,
    rows: Vec<Row>,
    next_row: u64,
    baseline: Vec<String>,
    revision: u64,
    pub(super) support: Option<Result<(), String>>,
    error: Option<String>,
    stale: bool,
    submitted: Option<(u64, Vec<String>)>,
    result: Option<Completion>,
    dialog: bool,
    close_pending: bool,
    pub(super) teardown_uncertain: bool,
    completed: Option<u64>,
}
impl Session {
    pub(super) fn name(&self) -> String {
        self.context
            .as_ref()
            .map(|c| c.name.clone())
            .or_else(|| {
                self.project
                    .as_ref()
                    .and_then(|p| p.file_name())
                    .map(|p| p.to_string_lossy().into_owned())
            })
            .unwrap_or_else(|| "captured project".into())
    }
    fn values(&self) -> Result<Vec<String>, String> {
        self.rows.iter().map(Row::value).collect()
    }
    fn dirty(&self) -> bool {
        self.values().map_or(true, |values| values != self.baseline)
    }
    fn ready(&self) -> Result<(TestContext, Vec<String>), String> {
        if self.stale {
            return Err(
                "Refresh test context explicitly before running again. Arguments are retained."
                    .into(),
            );
        }
        let context = self
            .context
            .clone()
            .ok_or("Refresh to read the captured project's test context.")?;
        let values = self.values()?;
        context.invocation(&values).map_err(|e| e.to_string())?;
        Ok((context, values))
    }
    fn add(&mut self) -> u64 {
        self.next_row += 1;
        self.rows.push(Row {
            id: self.next_row,
            text: String::new(),
            mode: Mode::Literal,
            empty: false,
            last_valid: String::new(),
        });
        self.next_row
    }
    fn changed(&mut self) {
        self.revision = self.revision.wrapping_add(1);
        self.error = None;
    }
}

#[derive(Default)]
pub(super) struct Inputs {
    rows: std::collections::BTreeMap<u64, Entity<InputState>>,
    subscriptions: std::collections::BTreeMap<u64, Subscription>,
}

fn watch_blocks(watch: Option<&WatchStatus>) -> bool {
    watch.is_some_and(|w| {
        matches!(
            w.state,
            rproj_core::operations::WatchState::Preparing
                | rproj_core::operations::WatchState::Stopping
                | rproj_core::operations::WatchState::Unresponsive
        )
    })
}

fn effect_text(effect: Effect) -> &'static str {
    match effect {
        Effect::ExecuteProjectTests => {
            "Run project-controlled tests. Only run projects you trust; options belong to the installed runner."
        }
        Effect::SynchronizeRokitTools => {
            "Synchronize Rokit tools; downloads and shared tool-cache changes may occur."
        }
        Effect::SynchronizeWallyDependencies => {
            "Synchronize Wally dependencies and generated types/sourcemaps as required."
        }
        Effect::RefreshJestConfiguration => {
            "Refresh Jest project/configuration and required Rojo mounts. Backend options can change prerequisites."
        }
    }
}
