use super::*;
use controller::Replace;
use gpui_kit::{
    assets::IconName,
    component::{button::*, input::Input},
    prelude::FluentBuilder,
};

impl Workbench {
    pub(in super::super) fn setup_entry(
        &self,
        id: &'static str,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let button = Button::new(id)
            .ghost()
            .icon(IconName::Wrench)
            .label("Machine Setup…")
            .selected(self.setup.visible)
            .disabled(!matches!(self.setup.support, Some(Ok(()))) && self.setup.context.is_none())
            .on_click(cx.listener(|this, _, window, cx| this.show_setup(window, cx)));
        div()
            .v_flex()
            .gap_1()
            .child(button)
            .when_some(
                self.setup
                    .support
                    .as_ref()
                    .and_then(|result| result.as_ref().err()),
                |view, error| view.child(div().text_sm().child(error.clone())),
            )
            .into_any_element()
    }
    pub(in super::super) fn render_setup(
        &self,
        stacked: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let state = &self.setup;
        let busy = self.running.is_some() || self.teardown_uncertain();
        let content = if state.loading && !self.setup_applying() {
            div()
                .child("Reading machine configuration and prerequisites…")
                .into_any_element()
        } else {
            match state.step {
                Step::Selections => self.setup_selections(stacked, cx),
                Step::Review => self.setup_review(cx),
                Step::Results => self.setup_results(cx),
            }
        };
        div().id("setup-flow").role(Role::Group).aria_label(format!("Machine Setup: {}",state.step.title())).test_support()
            .track_focus(&self.setup_focus).v_flex().size_full().min_h_0()
            .child(div().h_flex().flex_wrap().gap_3().px_6().py_2().children([Step::Selections,Step::Review,Step::Results].into_iter().map(|step| div().text_sm()
                .text_color(if step==state.step {cx.theme().foreground} else {cx.theme().muted_foreground}).child(step.title()))))
            .child(div().id("setup-scroll").role(Role::Group).aria_label("Scrollable Machine Setup").test_support().flex_1().min_h_0().overflow_y_scroll()
                .child(div().v_flex().gap_4().p_6()
                    .child("Machine-wide setup · independent of the selected project")
                    .when(self.running.is_some() && !self.setup_running(),|view|view.child("Another operation is running. Wait for its confirmed completion before reviewing or applying Machine Setup."))
                    .when_some(state.context.as_ref(),|view,context| view.child(div().id("setup-root").role(Role::Group).aria_label(format!("Projects folder: {}",context.saved.root.display())).test_support().min_w_0().overflow_x_scroll().whitespace_nowrap().text_sm().child(format!("Projects folder (read-only): {}",context.saved.root.display()))))
                    .when_some(state.error.clone(),|view,error|view.child(div().id("setup-error").role(Role::Alert).aria_label(error.clone()).test_support().text_color(cx.theme().danger).child(error)))
                    .when(state.teardown_uncertain,|view|view.child("Owned worker termination is unconfirmed. Closing and further mutations remain blocked; do not start a second setup. Installed software may remain."))
                    .when(state.reload_required,|view|view.child("Explicitly reload saved selections before another review. The previous result and Activity remain available until you choose to reload."))
                    .child(content)))
            .child(div().h_flex().flex_wrap().flex_none().gap_2().px_6().py_3().border_t_1().border_color(cx.theme().border)
                .child(Button::new("setup-discard").ghost().label(if state.result.is_some(){"Finish…"}else{"Discard draft…"}).disabled(busy).on_click(cx.listener(|this,_,window,cx|this.replace_setup(Replace::Discard(false),window,cx))))
                .child(Button::new("setup-reload").label("Reload saved selections…").disabled(busy || self.client.is_none()).on_click(cx.listener(|this,_,window,cx|this.replace_setup(Replace::Reload,window,cx))))
                .child(div().flex_1())
                .when(state.step==Step::Review,|view|view.child(Button::new("setup-back").label("Back").disabled(busy).on_click(cx.listener(|this,_,_,cx| {this.setup.step=Step::Selections;cx.notify();}))))
                .child(if self.setup_applying() {
                    Button::new("setup-stop").label("Stop setup…").disabled(state.stop.as_ref().is_some_and(SetupStop::requested)).on_click(cx.listener(|this,_,window,cx|this.confirm_setup_stop(false,window,cx))).into_any_element()
                } else if state.step==Step::Review {
                    Button::new("setup-apply").primary().label("Apply setup…").disabled(busy || state.ready().is_none() || SetupSession::watch_conflict(self.watch.as_ref())).on_click(cx.listener(|this,_,window,cx|this.confirm_setup_apply(window,cx))).into_any_element()
                } else {
                    Button::new("setup-review").primary().label(if state.step==Step::Results {"Reload and review…"}else{"Review selections"}).disabled(busy || !matches!(state.support,Some(Ok(()))) || state.selection.is_none() || (state.reload_required && state.step!=Step::Results)).on_click(cx.listener(|this,_,window,cx|{
                        if this.setup.step==Step::Results {this.replace_setup(Replace::Retry,window,cx);}else{this.prepare_machine_setup(cx);}
                    })).into_any_element()
                }))
            .into_any_element()
    }
    fn setup_selections(&self, stacked: bool, cx: &mut Context<Self>) -> AnyElement {
        let state = &self.setup;
        let Some(selection) = &state.selection else {
            return div().v_flex().gap_2().child("Machine Setup reads the existing machine configuration, or its default projects folder. No project record is required.")
                .child(state.support.as_ref().and_then(|s|s.as_ref().err()).cloned().unwrap_or_else(||"Connecting to a matching bundled CLI…".into())).into_any_element();
        };
        let category = state.category;
        let active = selection.active(category);
        let query = self.setup_query.read(cx).value().to_lowercase();
        let entries = category
            .entries()
            .into_iter()
            .filter(|e| {
                query.is_empty()
                    || format!("{} {} {}", e.key, e.description, e.family)
                        .to_lowercase()
                        .contains(&query)
            })
            .collect::<Vec<_>>();
        let disabled = self.running.is_some() || state.reload_required || self.teardown_uncertain();
        let choices=div().id("setup-choices").role(Role::Group).aria_label("Machine software choices").test_support().v_flex().gap_2().min_w_0().flex_1()
            .child(Input::new(&self.setup_query).id("setup-search").aria_label("Search setup choices"))
            .child("Saved selections are not installation evidence. Unchecking does not uninstall software.")
            .when(!active,|view|view.child(if category==Category::Blender {"Inactive: select Blender in System Apps. Retained add-ons are excluded from execution."}else{"Inactive: select VS Code in System Apps. Retained extensions/themes are excluded from execution."}))
            .when(entries.is_empty(),|view|view.child("No choices match this search. Selections are retained."))
            .children(entries.into_iter().map(|entry| {
                let key=entry.key; let selected=selection.keys(category).contains(key);
                let saved=state.context.as_ref().is_some_and(|c|c.saved.keys(category).contains(key));
                div().v_flex().gap_1().pb_2().border_b_1().border_color(cx.theme().border)
                    .child(div().h_flex().flex_wrap().gap_2()
                        .child(Button::new(SharedString::from(format!("setup-choice-{key}"))).label(key).selected(selected).disabled(disabled || !active)
                            .on_click(cx.listener(move|this,_,_,cx|{this.setup.toggle(category,key);cx.notify();})))
                        .child(Button::new(SharedString::from(format!("setup-details-{key}"))).ghost().label("Details").selected(state.selected.as_deref()==Some(key))
                            .on_click(cx.listener(move|this,_,_,cx|{this.setup.selected=Some(key.into());cx.notify();}))))
                    .child(div().text_sm().child(format!("{} · {}",if saved {"Recorded"}else{"Not recorded"},if selected {if active {"Selected"}else{"Selected / inactive"}}else{"Not selected"})))
            }));
        let unknown = Category::ALL
            .into_iter()
            .flat_map(|category| {
                selection
                    .keys(category)
                    .iter()
                    .filter(move |key| {
                        !Category::ALL
                            .into_iter()
                            .filter(|other| category.shares_storage(*other))
                            .flat_map(Category::entries)
                            .any(|e| e.key == key.as_str())
                    })
                    .cloned()
            })
            .collect::<std::collections::BTreeSet<_>>();
        let details = self.setup_details(cx);
        div()
            .v_flex()
            .gap_3()
            .child(
                div()
                    .h_flex()
                    .flex_wrap()
                    .gap_2()
                    .children(Category::ALL.into_iter().map(|cat| {
                        Button::new(SharedString::from(format!(
                            "setup-category-{}",
                            cat.label()
                        )))
                        .label(cat.label())
                        .selected(cat == category)
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.setup.category = cat;
                            this.setup.selected = None;
                            cx.notify();
                        }))
                    })),
            )
            .when(!unknown.is_empty(), |view| {
                view.child(format!(
                    "Unfamiliar recorded choices (retained, never executed): {}",
                    unknown.into_iter().collect::<Vec<_>>().join(", ")
                ))
            })
            .child(
                div()
                    .id(if stacked {
                        "setup-stacked"
                    } else {
                        "setup-adjacent"
                    })
                    .role(Role::Group)
                    .aria_label("Setup list and details")
                    .test_support()
                    .flex()
                    .gap_4()
                    .min_w_0()
                    .when(stacked, |view| view.flex_col())
                    .when(!stacked, |view| view.flex_row())
                    .child(choices)
                    .child(details),
            )
            .into_any_element()
    }
    fn setup_details(&self, _cx: &mut Context<Self>) -> AnyElement {
        let entry = self
            .setup
            .selected
            .as_deref()
            .and_then(tool_catalog::find)
            .or_else(|| self.setup.category.entries().first().copied());
        let Some(entry) = entry else {
            return div()
                .child("Select a choice to see its details")
                .into_any_element();
        };
        let detected = self
            .setup
            .context
            .as_ref()
            .and_then(|c| c.detected.get(entry.key))
            .map(|d| match d {
                Detection::Present => "Detected as present".into(),
                Detection::NotVerified(why) => format!("Not verified: {why}"),
            })
            .unwrap_or_else(|| "Installation state has not been verified".into());
        let docs = entry.docs_url;
        div().id("setup-details").role(Role::Group).aria_label(format!("Details for {}",entry.key)).test_support().v_flex().flex_1().min_w_0().gap_3()
            .child(div().text_lg().font_weight(FontWeight::SEMIBOLD).child(entry.key))
            .child(entry.description).child(format!("Source: {}",entry.kind.provider())).child(detected)
            .child(Button::new("setup-docs").ghost().icon(IconName::ExternalLink).label("Documentation").on_click(move|_,_,cx|cx.open_url(docs)))
            .when(matches!(entry.kind,ToolKind::StudioPluginViaCli{..}),|view|view.child("Requires Roblox Studio and the matching CLI tool. Preparation checks the prerequisites before Apply."))
            .when(matches!(entry.kind,ToolKind::BlenderAddon{..}),|view|view.child("Requires Blender selected. Account or other manual requirements remain separate instructions."))
            .when_some(match entry.kind {ToolKind::StudioPluginManual{install_url,..}=>Some(install_url),_=>None},|view,url|view
                .child("Manual Studio plugin: rproj records instructions; it does not claim to install this choice.")
                .child(Button::new("setup-manual-docs").label("Manual installation instructions").on_click(move|_,_,cx|cx.open_url(url))))
            .into_any_element()
    }
    fn setup_review(&self, cx: &mut Context<Self>) -> AnyElement {
        let Some(review) = &self.setup.prepared else {
            return div().child("Prepare a valid review before applying. Fix the explanation above, then return to selections or explicitly reload.").into_any_element();
        };
        let folder_exists = review
            .context
            .identities
            .iter()
            .any(|p| p.path == review.selection.root && p.identity.is_some());
        div().v_flex().gap_3()
            .child("Exact execution sequence")
            .children(review.items.iter().enumerate().map(|(i,item)|div().child(format!("{}. {}",i+1,item.label()))))
            .child(if folder_exists {"Projects folder already exists; it is retained."}else{"The projects folder is missing; setup creates it at its listed step."})
            .child("Rokit bootstrap is required. Selected software can involve downloads, global tool changes, installer/UAC dialogs and manual actions. No software is uninstalled. Retry runs the full reviewed sequence, not only failed items.")
            .child("Completed runs, including warnings/manual steps, save selections. Fatal or stopped runs do not. Installation results and configuration-save failure are reported separately; there is no rollback.")
            .when(SetupSession::watch_conflict(self.watch.as_ref()),|view|view.child(div().id("setup-watch-conflict").role(Role::Alert).aria_label("Watch blocks Machine Setup").test_support().text_color(cx.theme().danger)
                .child(format!("Apply is blocked by Watch for {}. Use the separate Stop control and wait for acknowledgement. Setup never stops or restarts Watch.",self.watch.as_ref().and_then(|w|w.project.as_ref()).map(|p|p.display().to_string()).unwrap_or_else(||"an unknown project".into())))))
            .into_any_element()
    }
    fn setup_results(&self, cx: &mut Context<Self>) -> AnyElement {
        let report = self
            .setup
            .result
            .as_ref()
            .or(self.setup.progress.as_deref());
        let statuses = report.map(|r| r.statuses.as_slice()).unwrap_or_default();
        let title = if self.setup_applying() {
            if self.setup.stop.as_ref().is_some_and(SetupStop::requested) {
                "Stop requested — waiting for the active item and owned processes"
            } else {
                "Machine Setup is running"
            }
        } else {
            match report.and_then(|r| r.status) {
                Some(SetupStatus::Succeeded) => "Run completed — inspect item results",
                Some(SetupStatus::Stopped) => "Setup stopped",
                Some(SetupStatus::Failed) => "Setup has failures",
                None if self.setup.error.is_some() && !self.setup.teardown_uncertain => {
                    "Setup could not start"
                }
                _ => "Completion is uncertain",
            }
        };
        let persistence = match report
            .and_then(|r| r.result.as_ref())
            .map(|r| &r.persistence)
        {
            Some(Persistence::Saved { .. }) => {
                "Selections saved, including any reported warnings or manual steps.".into()
            }
            Some(Persistence::Failed { message }) => {
                format!("Configuration was not saved: {message}")
            }
            Some(Persistence::NotSaved) => {
                "Selections were not saved. Completed installations may remain.".into()
            }
            None if self.setup_applying() => "Selection persistence has not been confirmed.".into(),
            None if self.setup.error.is_some() && !self.setup.teardown_uncertain => {
                "Setup did not start; proposed selections remain available.".into()
            }
            None => "File and installation state is uncertain. Explicitly reload before retrying."
                .into(),
        };
        div().v_flex().gap_3().child(div().id("setup-result").role(Role::Status).aria_label(title).test_support().child(title))
            .when(report.is_some_and(|r|r.stop_acknowledged) && self.setup_applying(),|view|view.child("Worker acknowledged Stop. The active item is allowed to finish."))
            .children(self.setup.prepared.iter().flat_map(|r|r.items.iter()).enumerate().map(|(i,item)| {
                let status=statuses.get(i).copied().unwrap_or(Status::Pending);
                div().text_color(if status==Status::Failed{cx.theme().danger}else{cx.theme().foreground})
                    .child(format!("{}. {} · {}",i+1,item.label(),match status {Status::Pending=>"Pending",Status::Running=>"Running",Status::Already=>"Already present",Status::Installed=>"Installed",Status::Manual=>"Manual action required",Status::Failed=>"Failed",Status::Skipped=>"Skipped"}))
            }))
            .child(persistence).child("Installed software can remain after failure or Stop. Activity keeps the Machine Setup output across project navigation; no rollback is promised.")
            .into_any_element()
    }
}
