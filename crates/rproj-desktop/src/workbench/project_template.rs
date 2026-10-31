use super::*;
use gpui_kit::component::input::TextareaState;
use rproj_core::project_template::{
    builtin_project_template,
    metadata::{Metadata, ValueKind},
    model::EditorModel,
    protocol::{LoadedTemplate, PreparedTemplate, TemplateOutcome, TemplateStatus},
    review::{TemplateAction, TemplateSource},
    values::{self, SettingKind},
};
use serde_json::Value;

mod controller;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod ui_tests;
mod view;

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum Page {
    #[default]
    Explorer,
    Json,
    Review,
}
#[derive(Clone)]
enum Field {
    Add,
    ChangeClass,
    Rename,
    Move,
    Property(String, ValueKind),
    Attribute(String, ValueKind),
    Setting(&'static str, SettingKind),
}
struct Draft {
    loaded: LoadedTemplate,
    model: Option<EditorModel>,
    raw: String,
    raw_base: String,
    selected: Vec<String>,
    field: Option<Field>,
    field_origin: (String, String),
    field_dirty: bool,
}
impl Draft {
    fn new(loaded: LoadedTemplate) -> Self {
        let model = loaded
            .problem
            .is_none()
            .then(|| loaded.source.value().and_then(EditorModel::new).ok())
            .flatten();
        let raw =
            loaded.source.original.clone().unwrap_or_else(|| {
                serde_json::to_string_pretty(&builtin_project_template()).unwrap()
            });
        Self {
            loaded,
            model,
            raw_base: raw.clone(),
            raw,
            selected: Vec::new(),
            field: None,
            field_origin: Default::default(),
            field_dirty: false,
        }
    }
    fn dirty(&self) -> bool {
        self.field_dirty
            || self.raw != self.raw_base
            || self
                .model
                .as_ref()
                .is_some_and(|model| model.is_dirty() || self.loaded.problem.is_some())
    }
    fn apply_json(&mut self) -> anyhow::Result<()> {
        let parsed = values::parse_explorer_json(&self.raw)?;
        if let Some(model) = &mut self.model {
            model.replace_from_json(parsed)?;
        } else {
            self.model = Some(EditorModel::new(parsed)?);
        }
        self.raw_base.clone_from(&self.raw);
        self.repair_selection();
        Ok(())
    }
    fn repair_selection(&mut self) {
        if let Some(model) = &self.model {
            while !self.selected.is_empty() && model.node(&self.selected).is_none() {
                self.selected.pop();
            }
        }
        self.field = None;
        self.field_dirty = false;
    }
}

#[derive(Clone)]
pub(super) enum Task {
    Load,
    Prepare(TemplateSource, TemplateAction),
    Execute(Box<PreparedTemplate>),
}
impl Task {
    fn operation(&self) -> Operation {
        match self {
            Self::Load => Operation::LoadTemplate,
            Self::Prepare(..) => Operation::PrepareTemplate,
            Self::Execute(p) => match p.review.action {
                TemplateAction::Save(_) => Operation::SaveTemplate,
                TemplateAction::Reset => Operation::ResetTemplate,
            },
        }
    }
}
pub(super) enum Response {
    Loaded(LoadedTemplate),
    Prepared(
        Box<PreparedTemplate>,
        Arc<Vec<rproj_core::upgrade::diff::Row>>,
        String,
    ),
    Executed(TemplateOutcome),
}
#[derive(Default)]
pub(super) struct Session {
    pub(super) visible: bool,
    pub(super) support: Option<Result<(), String>>,
    pub(super) teardown_uncertain: bool,
    draft: Option<Draft>,
    page: Page,
    review_return: Page,
    summary: String,
    prepared: Option<Box<PreparedTemplate>>,
    rows: Arc<Vec<rproj_core::upgrade::diff::Row>>,
    pending: Option<Task>,
    reload: bool,
    error: Option<String>,
    notice: Option<String>,
    dialog: bool,
    close_after: Option<u64>,
    last: Option<(u64, bool)>,
    revision: u64,
    sync_json: bool,
}
impl Session {
    fn changed(&mut self) {
        self.revision = self.revision.wrapping_add(1);
        self.prepared = None;
        self.rows = Arc::default();
        self.error = None;
        self.notice = None;
    }
    fn load(&mut self, loaded: LoadedTemplate) {
        let draft = Draft::new(loaded);
        self.page = if draft.model.is_some() {
            Page::Explorer
        } else {
            Page::Json
        };
        self.changed();
        self.reload = false;
        self.sync_json = true;
        self.draft = Some(draft);
    }
    fn editable(&self) -> bool {
        self.draft.is_some() && self.pending.is_none() && !self.reload && !self.teardown_uncertain
    }
}
pub(super) struct Inputs {
    focus: FocusHandle,
    diff_focus: FocusHandle,
    list: ListState,
    query: Entity<InputState>,
    value: Entity<InputState>,
    name: Entity<InputState>,
    json: Entity<TextareaState>,
    _subscriptions: Vec<Subscription>,
}
impl Inputs {
    pub(super) fn new(w: &mut Window, cx: &mut Context<Workbench>) -> Self {
        let query = cx.new(|cx| InputState::new(w, cx).placeholder("Find a class or property"));
        let value = cx.new(|cx| InputState::new(w, cx).placeholder("Value"));
        let name = cx.new(|cx| InputState::new(w, cx).placeholder("Attribute or instance name"));
        let json = cx.new(|cx| TextareaState::new(w, cx).rows(12));
        let field_value = cx.subscribe(&value, |this, _, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change) {
                this.template_field_changed(cx);
            }
        });
        let field_name = cx.subscribe(&name, |this, _, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change) {
                this.template_field_changed(cx);
            }
        });
        let search = cx.subscribe(&query, |_, _, _: &InputEvent, cx| cx.notify());
        let text = cx.subscribe(&json, |this, entity, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change)
                && this.template.page == Page::Json
                && this.template.editable()
            {
                let value = entity.read(cx).value().to_string();
                if let Some(draft) = &mut this.template.draft
                    && draft.raw != value
                {
                    draft.raw = value;
                    this.template.changed();
                    cx.notify();
                }
            }
        });
        Self {
            focus: cx.focus_handle(),
            diff_focus: cx.focus_handle().tab_stop(true),
            list: ListState::new(0, ListAlignment::Top, px(40.)),
            query,
            value,
            name,
            json,
            _subscriptions: vec![search, text, field_value, field_name],
        }
    }
}

fn semantic_summary(plan: &PreparedTemplate) -> String {
    let before = match plan.review.source.value() {
        Ok(value) => value,
        Err(_) => {
            return "The saved override contains invalid JSON. Review shows its original bytes."
                .into();
        }
    };
    let after = match &plan.review.action {
        TemplateAction::Reset => builtin_project_template(),
        TemplateAction::Save(v) => v.clone(),
    };
    fn walk(
        before: &Value,
        after: &Value,
        path: &str,
        counts: &mut [usize; 3],
        examples: &mut Vec<String>,
    ) {
        if before == after {
            return;
        }
        if let (Some(old), Some(new)) = (before.as_object(), after.as_object()) {
            for (key, value) in old {
                let name = format!("{path}/{key}");
                if let Some(next) = new.get(key) {
                    walk(value, next, &name, counts, examples)
                } else {
                    counts[1] += 1;
                    if examples.len() < 4 {
                        examples.push(format!("Removed {name}"));
                    }
                }
            }
            for key in new.keys().filter(|k| !old.contains_key(*k)) {
                counts[0] += 1;
                if examples.len() < 4 {
                    examples.push(format!("Added {path}/{key}"));
                }
            }
        } else {
            counts[2] += 1;
            if examples.len() < 4 {
                examples.push(format!("Changed {path}"));
            }
        }
    }
    let mut counts = [0; 3];
    let mut examples = Vec::new();
    walk(&before, &after, "", &mut counts, &mut examples);
    format!(
        "{} additions · {} removals · {} changed values. {}",
        counts[0],
        counts[1],
        counts[2],
        examples.join(" · ")
    )
}
