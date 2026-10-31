use super::*;
use controller::Intent;
use gpui_kit::component::menu::{DropdownMenu, PopupMenuItem};
use gpui_kit::component::{
    button::*,
    input::{Input, Textarea},
};
use gpui_kit::prelude::FluentBuilder;

#[derive(Clone)]
pub(super) enum Act {
    Show,
    Reload,
    Discard,
    Explorer,
    Json,
    ApplyJson,
    Review,
    Reset,
    Confirm,
    Undo,
    Redo,
    Node(Vec<String>),
    Field(Field),
    PickClass(String, bool),
    MoveTo(Vec<String>),
    Set,
    CancelField,
    Duplicate,
    Delete,
    RemoveProperty(String),
    RemoveAttribute(String),
    Ignore(bool),
}

fn display(value: Option<&Value>) -> String {
    match value {
        None => String::new(),
        Some(Value::String(text)) => text.clone(),
        Some(Value::Array(items)) => items
            .iter()
            .map(|item| display(Some(item)))
            .collect::<Vec<_>>()
            .join(", "),
        Some(Value::Object(map)) if map.len() == 1 => display(map.values().next()),
        Some(Value::Object(map))
            if map.contains_key("position") && map.contains_key("orientation") =>
        {
            format!(
                "{}, {}",
                display(map.get("position")),
                display(map.get("orientation"))
            )
        }
        Some(other) => other.to_string(),
    }
}
impl Workbench {
    pub(super) fn template_field_changed(&mut self, cx: &mut Context<Self>) {
        if self.template.page != Page::Explorer || !self.template.editable() {
            return;
        }
        if let Some(d) = &mut self.template.draft
            && d.field.is_some()
        {
            let value = self.template_inputs.value.read(cx).value();
            let name = self.template_inputs.name.read(cx).value();
            d.field_dirty = value.as_ref() != d.field_origin.0 || name.as_ref() != d.field_origin.1;
            cx.notify();
        }
    }

    fn template_field_blocks(&self, act: &Act) -> bool {
        self.template.page == Page::Explorer
            && self.template.draft.as_ref().is_some_and(|d| d.field_dirty)
            && !matches!(
                act,
                Act::Set | Act::CancelField | Act::Show | Act::Discard | Act::Reload | Act::Reset
            )
    }
    fn template_button(
        &self,
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
        act: Act,
        disabled: bool,
        cx: &mut Context<Self>,
    ) -> Button {
        let disabled = disabled || self.template_field_blocks(&act);
        Button::new(id.into())
            .label(label.into())
            .disabled(disabled)
            .on_click(cx.listener(move |this, _, w, cx| this.template_action(act.clone(), w, cx)))
    }
    pub(in super::super) fn template_entry(&self, cx: &mut Context<Self>) -> AnyElement {
        self.template_button(
            "project-template",
            "Project Template…",
            Act::Show,
            false,
            cx,
        )
        .w_full()
        .ghost()
        .icon(IconName::FolderOpen)
        .selected(self.template.visible)
        .into_any_element()
    }
    pub(super) fn template_action(&mut self, action: Act, w: &mut Window, cx: &mut Context<Self>) {
        if self.template_field_blocks(&action) {
            return;
        }
        match action {
            Act::Show => self.show_template(w, cx),
            Act::Reload => self.template_decision(Intent::Reload, w, cx),
            Act::Discard => self.template_decision(Intent::Discard, w, cx),
            Act::Confirm => self.template_decision(Intent::Execute, w, cx),
            Act::Delete => self.template_decision(Intent::Delete, w, cx),
            Act::Review => self.prepare_template(false, cx),
            Act::Reset => self.prepare_template(true, cx),
            act if self.template.editable() => {
                let text = self.template_inputs.value.read(cx).value().to_string();
                let name = self.template_inputs.name.read(cx).value().to_string();
                match act {
                    Act::Explorer => {
                        if self.template.page == Page::Review {
                            self.template.page = self.template.review_return;
                            self.template.changed();
                        }
                    }
                    Act::Json => {
                        let d = self.template.draft.as_mut().unwrap();
                        if self.template.page != Page::Json {
                            if let Some(model) = &d.model {
                                d.raw = serde_json::to_string_pretty(model.value()).unwrap();
                                d.raw_base.clone_from(&d.raw);
                            }
                            self.template.page = Page::Json;
                            self.template.sync_json = true;
                            self.template.changed();
                        }
                    }
                    Act::ApplyJson => {
                        let raw = self.template_inputs.json.read(cx).value().to_string();
                        self.edit_template(
                            |d| {
                                d.raw = raw;
                                d.apply_json()
                            },
                            cx,
                        );
                        if self.template.error.is_none() {
                            self.template.page = Page::Explorer;
                        }
                    }
                    Act::Node(path) => {
                        let d = self.template.draft.as_mut().unwrap();
                        d.selected = path;
                        d.field = None;
                    }
                    Act::Field(field) => {
                        let d = self.template.draft.as_mut().unwrap();
                        let Some(model) = &d.model else { return };
                        let props = model.node(&d.selected).and_then(|n| n.get("$properties"));
                        let current = match &field {
                            Field::Property(key, _) => display(props.and_then(|p| p.get(key))),
                            Field::Attribute(key, _) => display(
                                props
                                    .and_then(|p| p.get("Attributes"))
                                    .and_then(|p| p.get(key))
                                    .and_then(values::explicit_inner),
                            ),
                            Field::Rename => d.selected.last().cloned().unwrap_or_default(),
                            Field::Setting(key, _) => display(model.value().get(key)),
                            _ => String::new(),
                        };
                        let initial_name = match &field {
                            Field::Attribute(key, _) => key.clone(),
                            _ => String::new(),
                        };
                        d.field_origin = (current.clone(), initial_name);
                        d.field_dirty = false;
                        self.template_inputs
                            .value
                            .update(cx, |input, cx| input.set_value(current, w, cx));
                        self.template_inputs.name.update(cx, |input, cx| {
                            input.set_value(
                                match &field {
                                    Field::Attribute(key, _) => key.clone(),
                                    _ => String::new(),
                                },
                                w,
                                cx,
                            )
                        });
                        d.field = Some(field);
                    }
                    Act::CancelField => {
                        let d = self.template.draft.as_mut().unwrap();
                        d.field = None;
                        d.field_dirty = false;
                    }
                    act => self.edit_template(
                        |d| {
                            let m = d.model.as_mut().ok_or_else(|| {
                                anyhow::anyhow!("Repair the template in Advanced JSON")
                            })?;
                            match act {
                                Act::Undo => {
                                    m.undo();
                                }
                                Act::Redo => {
                                    m.redo();
                                }
                                Act::Duplicate => d.selected = m.duplicate(&d.selected)?,
                                Act::Ignore(value) => m.set_ignore_unknown(&d.selected, value)?,
                                Act::RemoveProperty(key) => m.remove_property(&d.selected, &key)?,
                                Act::RemoveAttribute(key) => {
                                    m.remove_attribute(&d.selected, &key)?
                                }
                                Act::PickClass(class, service) => {
                                    d.selected = match d.field {
                                        Some(Field::Add) => m.add(&d.selected, &class, service)?,
                                        Some(Field::ChangeClass) => {
                                            m.change_class(&d.selected, &class, service)?
                                        }
                                        _ => {
                                            anyhow::bail!("Choose Add child or Change class first")
                                        }
                                    }
                                }
                                Act::MoveTo(path) => d.selected = m.move_to(&d.selected, &path)?,
                                Act::Set => match d.field.clone() {
                                    Some(Field::Rename) => {
                                        d.selected = m.rename(&d.selected, &text)?
                                    }
                                    Some(Field::Property(key, kind)) => m.set_property(
                                        &d.selected,
                                        &key,
                                        kind.parse(&text).map_err(anyhow::Error::msg)?,
                                    )?,
                                    Some(Field::Attribute(key, kind)) => {
                                        let name = if key.is_empty() { name.trim() } else { &key };
                                        anyhow::ensure!(
                                            !name.is_empty(),
                                            "Enter an attribute name"
                                        );
                                        let value =
                                            kind.parse(&text).map_err(anyhow::Error::msg)?;
                                        values::set_attribute(
                                            m,
                                            &d.selected,
                                            name,
                                            values::attribute_value(&kind, value),
                                        )?;
                                    }
                                    Some(Field::Setting(key, kind)) => m.set_setting(
                                        key,
                                        values::parse_setting(kind, &text)
                                            .map_err(anyhow::Error::msg)?,
                                    )?,
                                    _ => anyhow::bail!("Select a field to edit"),
                                },
                                _ => return Ok(()),
                            }
                            d.repair_selection();
                            Ok(())
                        },
                        cx,
                    ),
                }
            }
            _ => {}
        }
        cx.notify();
    }
    pub(in super::super) fn render_template(
        &mut self,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        if self.template.sync_json
            && let Some(d) = &self.template.draft
        {
            self.template.sync_json = false;
            self.template_inputs
                .json
                .update(cx, |input, cx| input.set_value(d.raw.clone(), w, cx));
        }
        let narrow =
            w.viewport_size().width - theme::SIDEBAR_WIDTH.to_pixels(w.rem_size()) < px(600.);
        let busy = self.running.is_some() || self.teardown_uncertain();
        let mut body = div()
            .id("template-flow")
            .overflow_y_scroll()
            .role(Role::Group)
            .aria_label("Machine-wide Project Template")
            .test_support()
            .track_focus(&self.template_inputs.focus)
            .v_flex()
            .size_full()
            .min_h_0()
            .min_w_0()
            .gap_3()
            .p_4();
        let Some(Ok(())) = &self.template.support else {
            let reason = self
                .template
                .support
                .as_ref()
                .and_then(|r| r.as_ref().err())
                .cloned()
                .or_else(|| self.client_error.clone())
                .unwrap_or_else(|| "Connecting to the matching Project Template worker…".into());
            return body.child(reason).into_any_element();
        };
        let idle = self.template.editable() && !busy;
        let identity = self
            .template
            .draft
            .as_ref()
            .map(|d| {
                format!(
                    "{} · {}",
                    if d.loaded.source.original.is_some() {
                        "Custom override"
                    } else {
                        "Built-in template"
                    },
                    d.loaded.source.path.display()
                )
            })
            .unwrap_or_else(|| "Machine-wide template for future projects".into());
        body = body
            .child(
                div()
                    .id("template-path")
                    .role(Role::Group)
                    .aria_label(identity.clone())
                    .test_support()
                    .overflow_x_scroll()
                    .whitespace_nowrap()
                    .child(identity),
            )
            .child("Changes apply to future projects.")
            .child(
                div()
                    .h_flex()
                    .flex_wrap()
                    .gap_2()
                    .child(self.template_button(
                        "template-reload",
                        "Reload from file",
                        Act::Reload,
                        busy,
                        cx,
                    ))
                    .child(self.template_button(
                        "template-discard",
                        "Discard edits…",
                        Act::Discard,
                        busy || !self.template.draft.as_ref().is_some_and(Draft::dirty),
                        cx,
                    )),
            );
        if let Some(e) = &self.template.error {
            body = body.child(
                div()
                    .id("template-error")
                    .role(Role::Alert)
                    .text_color(cx.theme().danger)
                    .child(e.clone()),
            );
        }
        if let Some(s) = &self.template.notice {
            body = body.child(s.clone());
        }
        if self.template.reload {
            body = body.child("Reload from file before reviewing or saving again.");
        }
        if let Some(task) = &self.template.pending {
            body = body.child(format!(
                "{}…",
                super::super::operation_label(task.operation())
            ));
        }
        let Some(d) = &self.template.draft else {
            return body.into_any_element();
        };
        let reset_disabled = !idle || d.loaded.source.original.is_none();
        if self.template.page == Page::Review
            && let Some(plan) = &self.template.prepared
        {
            let reset = matches!(plan.review.action, TemplateAction::Reset);
            body = body
                .child(if reset {
                    "Review removal of the custom template"
                } else {
                    "Review template changes"
                })
                .child(if reset {
                    "The override file will be removed. Pending edits are discarded after success."
                } else {
                    "The override is written as formatted JSON. Review every changed line below."
                });
            if reset {
                let built_in =
                    EditorModel::new(builtin_project_template()).expect("valid built-in template");
                let tree = built_in
                    .rows()
                    .into_iter()
                    .filter(|row| !row.path.is_empty())
                    .map(|row| row.path.join(" / "))
                    .collect::<Vec<_>>()
                    .join("\n");
                body = body.child(
                    div()
                        .id("template-reset-tree")
                        .role(Role::Group)
                        .aria_label("Resulting built-in tree")
                        .test_support()
                        .max_h(px(80.))
                        .overflow_y_scroll()
                        .child(format!("Resulting built-in tree\n{tree}")),
                );
            }
            body = body.child(self.template.summary.clone());
            if !plan.review.changes() {
                body = body.child("No changes. The template file will be left as it is.");
            }
            return body
                .child(
                    div()
                        .h_flex()
                        .flex_wrap()
                        .gap_2()
                        .child(self.template_button(
                            "template-back",
                            "Back to editor",
                            Act::Explorer,
                            !idle,
                            cx,
                        ))
                        .child(self.template_button(
                            "template-execute",
                            if reset {
                                "Restore built-in template…"
                            } else {
                                "Save template…"
                            },
                            Act::Confirm,
                            !idle || !plan.review.changes(),
                            cx,
                        )),
                )
                .child(super::super::diff_view::render(
                    "template",
                    self.template.rows.clone(),
                    self.template_inputs.list.clone(),
                    self.template_inputs.diff_focus.clone(),
                    false,
                    cx,
                ))
                .into_any_element();
        }
        let mut actions = div()
            .h_flex()
            .flex_wrap()
            .gap_2()
            .child(self.template_button(
                "template-reset",
                "Review restore built-in…",
                Act::Reset,
                reset_disabled,
                cx,
            ));
        if self.template.page == Page::Json {
            if let Some(problem) = &d.loaded.problem {
                body = body.child(format!("Override needs repair: {problem}"));
            }
            return body
                .child(actions.child(self.template_button(
                    "template-apply-json",
                    "Apply to draft",
                    Act::ApplyJson,
                    !idle,
                    cx,
                )))
                .child("Advanced JSON · Apply valid text to return to Explorer.")
                .child(
                    Textarea::new(&self.template_inputs.json)
                        .disabled(!idle)
                        .h_full(),
                )
                .into_any_element();
        }
        actions = actions
            .child(self.template_button("template-json", "Advanced JSON", Act::Json, !idle, cx))
            .child(self.template_button(
                "template-review",
                "Review changes…",
                Act::Review,
                !idle,
                cx,
            ))
            .child(self.template_button(
                "template-undo",
                "Undo",
                Act::Undo,
                !idle || !d.model.as_ref().is_some_and(EditorModel::can_undo),
                cx,
            ))
            .child(self.template_button(
                "template-redo",
                "Redo",
                Act::Redo,
                !idle || !d.model.as_ref().is_some_and(EditorModel::can_redo),
                cx,
            ));
        body = body.child(actions);
        let Some(model) = &d.model else {
            return body.into_any_element();
        };
        let mut tree = div()
            .id("template-tree")
            .v_flex()
            .gap_1()
            .min_w_0()
            .child("Explorer")
            .child(
                self.template_button(
                    "template-tree-root",
                    "DataModel",
                    Act::Node(vec![]),
                    !idle,
                    cx,
                )
                .selected(d.selected.is_empty()),
            );
        for (n, row) in model
            .rows()
            .into_iter()
            .filter(|row| !row.path.is_empty())
            .enumerate()
        {
            tree = tree.child(
                self.template_button(
                    format!("template-node-{n}"),
                    format!(
                        "{}{}{}",
                        "  ".repeat(row.depth),
                        row.name,
                        if row.locked { " · protected" } else { "" }
                    ),
                    Act::Node(row.path.clone()),
                    !idle,
                    cx,
                )
                .ghost()
                .selected(d.selected == row.path),
            );
        }
        let inspector = self.template_inspector(d, model, !idle, cx);
        let panes = if narrow {
            div().id("template-stacked").v_flex()
        } else {
            div().id("template-adjacent").h_flex().items_start()
        }
        .test_support()
        .w_full()
        .gap_4()
        .child(tree.when(!narrow, |t| t.flex_1()))
        .child(inspector.when(!narrow, |t| t.flex_1()));
        body.child(
            div()
                .id("template-explorer-scroll")
                .flex_1()
                .min_h_0()
                .overflow_y_scroll()
                .child(panes),
        )
        .into_any_element()
    }
}

impl Workbench {
    fn template_inspector(
        &self,
        d: &Draft,
        m: &EditorModel,
        locked: bool,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let meta = Metadata::bundled();
        let class = m.class_name(&d.selected);
        let path = if d.selected.is_empty() {
            "DataModel".into()
        } else {
            d.selected.join(" / ")
        };
        let node = m.node(&d.selected);
        let props = node.and_then(|n| n.get("$properties"));
        let query = self.template_inputs.query.read(cx).value().to_lowercase();
        let find = |text: &str| text.to_lowercase().contains(&query);
        let protected = !m.can_restructure(&d.selected);
        let mount = !m.can_add_children(&d.selected);
        let mut out = div()
            .id("template-inspector")
            .v_flex()
            .min_w_0()
            .gap_2()
            .child(format!("{path} · {class}"));
        let incompatible = m.incompatible_properties(&d.selected, &meta);
        if !incompatible.is_empty() {
            out=out.child(format!("Retained properties need repair for {class}: {}. Remove a property explicitly or repair it in Advanced JSON.",incompatible.join(", ")));
        }
        out = out.child(
            div()
                .h_flex()
                .flex_wrap()
                .gap_2()
                .child(self.template_button(
                    "template-add",
                    "Add child",
                    Act::Field(Field::Add),
                    locked || !m.can_add_children(&d.selected),
                    cx,
                ))
                .child(self.template_button(
                    "template-class",
                    "Change class",
                    Act::Field(Field::ChangeClass),
                    locked || protected,
                    cx,
                ))
                .child(self.template_button(
                    "template-rename",
                    "Rename",
                    Act::Field(Field::Rename),
                    locked || protected,
                    cx,
                ))
                .child(self.template_button(
                    "template-duplicate",
                    "Duplicate",
                    Act::Duplicate,
                    locked || protected,
                    cx,
                ))
                .child(self.template_button(
                    "template-move",
                    "Move to…",
                    Act::Field(Field::Move),
                    locked || protected,
                    cx,
                ))
                .child(self.template_button(
                    "template-delete",
                    "Delete…",
                    Act::Delete,
                    locked || protected,
                    cx,
                )),
        );
        if protected {
            out =
                out.child("Required nodes and managed mounts cannot be renamed, moved or deleted.");
        }
        out = out.child(Input::new(&self.template_inputs.query).disabled(locked));
        if let Some(field) = &d.field {
            let mut pane = div().v_flex().gap_2();
            match field {
                Field::Add | Field::ChangeClass => {
                    let at_root = if matches!(field, Field::Add) {
                        d.selected.is_empty()
                    } else {
                        d.selected.len() == 1
                    };
                    let items: Vec<_> = meta
                        .classes(at_root)
                        .into_iter()
                        .filter(|c| find(&c.name))
                        .collect();
                    pane = pane.child(format!(
                        "Choose class · showing {} of {} matches. Search to narrow the list.",
                        items.len().min(24),
                        items.len()
                    ));
                    for (i, c) in items.into_iter().take(24).enumerate() {
                        pane = pane.child(self.template_button(
                            format!("template-pick-{i}"),
                            c.name.clone(),
                            Act::PickClass(c.name, c.service),
                            locked,
                            cx,
                        ));
                    }
                }
                Field::Move => {
                    pane = pane
                        .child("Choose a destination")
                        .child(self.template_button(
                            "template-move-root",
                            "DataModel",
                            Act::MoveTo(vec![]),
                            locked,
                            cx,
                        ));
                    for (i, row) in m.rows().into_iter().enumerate().filter(|(_, r)| {
                        !r.path.is_empty()
                            && m.can_add_children(&r.path)
                            && !r.path.starts_with(&d.selected)
                            && find(&r.path.join("/"))
                    }) {
                        pane = pane.child(self.template_button(
                            format!("template-move-{i}"),
                            row.path.join(" / "),
                            Act::MoveTo(row.path),
                            locked,
                            cx,
                        ));
                    }
                }
                field => {
                    let hint = match field {
                        Field::Property(key, kind) => format!("{key} · {}", kind.label()),
                        Field::Attribute(key, kind) => {
                            format!("Attribute {key} · {}", kind.label())
                        }
                        Field::Setting(key, _) => {
                            format!("Project setting {key} · empty removes the setting")
                        }
                        _ => "New node name".into(),
                    };
                    pane = pane.child(hint);
                    let choices = match field {
                        Field::Property(_, ValueKind::Enum(items))
                        | Field::Attribute(_, ValueKind::Enum(items)) => items.clone(),
                        Field::Property(_, ValueKind::Bool)
                        | Field::Attribute(_, ValueKind::Bool)
                        | Field::Setting(_, SettingKind::Bool) => {
                            vec!["true".into(), "false".into()]
                        }
                        _ => Vec::new(),
                    };
                    if !choices.is_empty() {
                        let input = self.template_inputs.value.clone();
                        pane = pane.child(
                            Button::new("template-value-choices")
                                .label("Choose value…")
                                .disabled(locked)
                                .dropdown_menu(move |mut menu, _, _| {
                                    for choice in &choices {
                                        let state = input.clone();
                                        let value = choice.clone();
                                        menu =
                                            menu.item(PopupMenuItem::new(value.clone()).on_click(
                                                move |_, w, cx| {
                                                    state.update(cx, |input, cx| {
                                                        input.replace_all(value.clone(), w, cx)
                                                    });
                                                },
                                            ));
                                    }
                                    menu
                                }),
                        );
                    }
                    if matches!(field,Field::Attribute(key,_) if key.is_empty()) {
                        pane = pane.child(Input::new(&self.template_inputs.name).disabled(locked));
                    }
                    pane = pane
                        .child(Input::new(&self.template_inputs.value).disabled(locked))
                        .child(self.template_button(
                            "template-set",
                            "Apply field to draft",
                            Act::Set,
                            locked,
                            cx,
                        ));
                }
            }
            if d.field_dirty {
                pane = pane
                    .child("Apply or discard this field before editing another part of the draft.");
            }
            return out.child(pane.child(self.template_button(
                "template-cancel-field",
                if d.field_dirty {
                    "Discard field input"
                } else {
                    "Back to inspector"
                },
                Act::CancelField,
                locked,
                cx,
            )));
        }
        out = out.child("Properties");
        if let Some(props) = props.and_then(Value::as_object) {
            for (i, (key, val)) in props
                .iter()
                .enumerate()
                .filter(|(_, (key, _))| key.as_str() != "Attributes")
            {
                out = out.child(
                    div()
                        .h_flex()
                        .flex_wrap()
                        .gap_2()
                        .child(format!("{key} = {val}"))
                        .child(self.template_button(
                            format!("template-drop-property-{i}"),
                            "Remove",
                            Act::RemoveProperty(key.clone()),
                            locked || mount,
                            cx,
                        )),
                );
            }
        }
        let available: Vec<_> = meta
            .properties(&class)
            .into_iter()
            .filter(|p| find(&p.name))
            .collect();
        out = out.child(format!(
            "Set a property · showing {} of {} matches",
            available.len().min(12),
            available.len()
        ));
        let mut choices = div().h_flex().flex_wrap().gap_2();
        for (i, p) in available.into_iter().take(12).enumerate() {
            choices = choices.child(self.template_button(
                format!("template-property-{i}"),
                p.name.clone(),
                Act::Field(Field::Property(p.name, p.kind)),
                locked || mount,
                cx,
            ));
        }
        out = out.child(choices).child("Attributes");
        if let Some(attrs) = props
            .and_then(|v| v.get("Attributes"))
            .and_then(Value::as_object)
        {
            for (i, (key, val)) in attrs.iter().enumerate() {
                let mut row = div()
                    .h_flex()
                    .flex_wrap()
                    .gap_2()
                    .child(format!("{key} = {val}"));
                if let Some(kind) = values::attribute_kind(val) {
                    row = row.child(self.template_button(
                        format!("template-attribute-edit-{i}"),
                        "Edit",
                        Act::Field(Field::Attribute(key.clone(), kind)),
                        locked || mount,
                        cx,
                    ));
                }
                out = out.child(row.child(self.template_button(
                    format!("template-attribute-remove-{i}"),
                    "Remove",
                    Act::RemoveAttribute(key.clone()),
                    locked || mount,
                    cx,
                )));
            }
        }
        let mut types = div().h_flex().flex_wrap().gap_2();
        for (i, (label, kind)) in values::attribute_kinds().into_iter().enumerate() {
            types = types.child(self.template_button(
                format!("template-attribute-add-{i}"),
                format!("Add {label}"),
                Act::Field(Field::Attribute(String::new(), kind)),
                locked || mount,
                cx,
            ));
        }
        let ignore = node
            .and_then(|n| n.get("$ignoreUnknownInstances"))
            .and_then(Value::as_bool)
            .unwrap_or(false);
        out = out
            .child(types)
            .child(self.template_button(
                "template-ignore",
                format!("Ignore unknown instances: {ignore}"),
                Act::Ignore(!ignore),
                locked || mount,
                cx,
            ))
            .child("Project settings");
        for (i, &(key, label, kind)) in values::SETTINGS.iter().enumerate() {
            let current = m
                .value()
                .get(key)
                .map(|v| v.to_string())
                .unwrap_or_else(|| "unset".into());
            out = out.child(self.template_button(
                format!("template-setting-{i}"),
                format!("{label} · {current}"),
                Act::Field(Field::Setting(key, kind)),
                locked,
                cx,
            ));
        }
        out
    }
}
