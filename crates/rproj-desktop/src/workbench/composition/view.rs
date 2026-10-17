use super::*;
use gpui_kit::assets::IconName;
use gpui_kit::{
    component::{button::*, checkbox::Checkbox, input::Input},
    prelude::FluentBuilder,
};
use rproj_core::{
    catalog::{capabilities, wally_packages},
    creation::{PackageWorkflow, capability_owned, offerable_package},
};

impl Workbench {
    pub(in super::super) fn composition_entry(&self, cx: &mut Context<Self>) -> AnyElement {
        let unavailable = self.client.is_none()
            || self.composition.unavailable
            || self.composition.teardown_uncertain;
        div().h_flex().flex_wrap().gap_2().px_6().py_2()
            .child(Button::new("edit-composition").icon(IconName::Package).label("Edit packages & capabilities…")
                .disabled(unavailable || self.project.is_none() || self.composition_running() || self.composition.loading)
                .on_click(cx.listener(|this, _, window, cx| this.edit_composition(window, cx))))
            .when(self.composition.project.is_some(), |view| view.child(Button::new("resume-composition")
                .label(format!("Resume editing {}", self.composition.context.as_ref().map(|c| c.name.as_str()).unwrap_or("captured project")))
                .on_click(cx.listener(|this, _, window, cx| this.show_composition(window, cx)))))
            .when(unavailable, |view| view.child(div().text_sm().child(self.composition.error.clone().unwrap_or_else(|| "Composition editing needs a matching bundled CLI; existing workbench actions remain available".into()))))
            .into_any_element()
    }
    pub(in super::super) fn render_composition(
        &self,
        stacked: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let state = &self.composition;
        let busy = self.running.is_some() || state.teardown_uncertain;
        let finished = state.result.is_some();
        let content = if state.loading {
            div().child("Reading captured project…").into_any_element()
        } else {
            match state.step {
                Step::Dependencies => self.composition_dependencies(stacked, cx),
                Step::Capabilities => self.composition_capabilities(cx),
                Step::Review => self.composition_review(cx),
            }
        };
        div().id("composition-flow").role(Role::Group).aria_label(format!("Edit composition: {}", state.step.title())).test_support()
            .track_focus(&self.composition_inputs.focus).v_flex().size_full().min_h_0()
            .child(div().h_flex().flex_wrap().gap_3().px_6().py_2().children(Step::ALL.into_iter().map(|step| div().text_sm()
                .text_color(if step == state.step { cx.theme().foreground } else { cx.theme().muted_foreground })
                .child(format!("{}. {}", step.index() + 1, step.title())))))
            .child(div().id("composition-scroll").role(Role::Group).aria_label("Scrollable composition step").test_support().flex_1().min_h_0().overflow_y_scroll()
                .child(div().v_flex().gap_4().p_6()
                    .child(div().id("composition-project-path").role(Role::Group).aria_label(state.project.as_ref().map(|p| p.display().to_string()).unwrap_or_default()).test_support().min_w_0().overflow_x_scroll().whitespace_nowrap().text_sm().child(format!("Captured project: {}", state.project.as_ref().map(|p| p.display().to_string()).unwrap_or_default())))
                    .child("Existing packages, implementations, workflow and file exclusions are locked. New choices can be revised before Apply.")
                    .when_some(state.error.clone(), |view, error| view.child(div().id("composition-error").role(Role::Alert).aria_label(error.clone()).test_support().text_color(cx.theme().danger).child(error)))
                    .child(content)))
            .child(div().h_flex().flex_none().flex_wrap().gap_2().px_6().py_3().border_t_1().border_color(cx.theme().border)
                .child(Button::new("composition-discard").ghost().label(if finished { "Finish" } else { "Discard draft…" }).disabled(busy || state.loading).on_click(cx.listener(|this, _, window, cx| this.discard_composition(window, cx))))
                .child(Button::new("composition-reload").label("Reload…").disabled(busy || state.loading || self.client.is_none()).on_click(cx.listener(|this, _, window, cx| this.reload_composition(window, cx))))
                .child(div().flex_1())
                .child(Button::new("composition-back").label("Back").disabled(busy || finished || state.step == Step::Dependencies).on_click(cx.listener(|this, _, window, cx| {
                    if this.running.is_none() && this.composition.step != Step::Dependencies { this.composition.step = Step::ALL[this.composition.step.index() - 1]; this.focus_composition(window, cx); cx.notify(); }
                })))
                .child(if state.step == Step::Review {
                    div().h_flex().flex_wrap().gap_2()
                        .child(Button::new("composition-prepare").label("Prepare review").disabled(busy || finished || state.loading).on_click(cx.listener(|this, _, window, cx| this.composition_next(window, cx))))
                        .child(Button::new("composition-apply").primary().label("Apply additions…").disabled(busy || finished || !state.dirty || state.fresh_plan().is_none() || state.watch_conflict(self.watch.as_ref())).on_click(cx.listener(|this, _, window, cx| this.confirm_composition_apply(window, cx))))
                        .into_any_element()
                } else {
                    Button::new("composition-next").primary().label("Next").disabled(busy || finished || state.ready().is_err()).on_click(cx.listener(|this, _, window, cx| this.composition_next(window, cx))).into_any_element()
                }))
            .into_any_element()
    }
    fn composition_dependencies(&self, stacked: bool, cx: &mut Context<Self>) -> AnyElement {
        let Some(draft) = &self.composition.draft else {
            return div().child("A valid recorded composition is required. Repair the record, then Reload; this editor does not adopt projects.").into_any_element();
        };
        let busy = self.composition_running()
            || self.composition.result.is_some()
            || self
                .composition
                .context
                .as_ref()
                .is_some_and(|c| c.problem.is_some());
        let query = self
            .composition_inputs
            .query
            .read(cx)
            .value()
            .to_lowercase();
        let mut list = div()
            .id("composition-package-list")
            .role(Role::Group)
            .aria_label("Package additions")
            .test_support()
            .v_flex()
            .gap_2()
            .flex_1()
            .min_w_0()
            .min_h_0()
            .max_h(if stacked { rems(10.) } else { rems(24.) })
            .overflow_y_scroll()
            .child(format!(
                "Dependency workflow: {:?} · Locked",
                draft.original.package_workflow
            ))
            .child(
                Input::new(&self.composition_inputs.query)
                    .id("composition-query")
                    .aria_label("Search package additions"),
            );
        if draft.graph.package_workflow == PackageWorkflow::None {
            list = list.child("This project uses no dependency manager. Capabilities remain available; dependency migration is outside this editor.");
        }
        for key in &draft.original.packages {
            if wally_packages::find(key).is_none() {
                list = list.child(format!(
                    "Recorded and locked: {key} · Unsupported; Apply is blocked"
                ));
            }
        }
        for package in wally_packages::PACKAGES {
            let key = package.key;
            let locked = draft.original.packages.iter().any(|p| p == key);
            if (!offerable_package(package, draft.graph.package_workflow) && !locked)
                || (!query.is_empty()
                    && !format!("{key} {}", package.description)
                        .to_lowercase()
                        .contains(&query))
            {
                continue;
            }
            list = list.child(
                div()
                    .v_flex()
                    .gap_1()
                    .pb_2()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(
                        Checkbox::new(SharedString::from(format!("composition-package-{key}")))
                            .label(format!(
                                "{key}{}",
                                if locked { " · Recorded / locked" } else { "" }
                            ))
                            .checked(draft.graph.packages.iter().any(|p| p == key))
                            .disabled(busy || locked)
                            .on_change(cx.listener(move |this, _, _, cx| {
                                this.mutate_composition(|draft| draft.toggle_package(key), cx)
                            })),
                    )
                    .child(
                        Button::new(SharedString::from(format!("composition-details-{key}")))
                            .ghost()
                            .label(format!("Details: {}", package.module_name))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.composition.selected_package = Some(key.into());
                                cx.notify();
                            })),
                    ),
            );
        }
        let mut details = div()
            .id("composition-package-details")
            .role(Role::Group)
            .aria_label("Package details and declarations")
            .test_support()
            .v_flex()
            .gap_3()
            .flex_1()
            .min_w_0()
            .min_h_0()
            .max_h(if stacked { rems(12.) } else { rems(28.) })
            .overflow_y_scroll();
        details = details.child(format!(
            "Recorded capabilities / locked: {}",
            draft
                .original
                .capabilities
                .iter()
                .map(|(key, implementation)| format!("{key}: {implementation}"))
                .collect::<Vec<_>>()
                .join(", ")
        ));
        if let Some(package) = self
            .composition
            .selected_package
            .as_deref()
            .and_then(wally_packages::find)
        {
            details = details
                .child(
                    div()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(package.module_name),
                )
                .child(package.description)
                .child(format!(
                    "Catalog default declaration: {} (existing declarations are preserved)",
                    package.source
                ))
                .child(
                    gpui_kit::base::Link::new("composition-package-docs")
                        .href(package.docs_url)
                        .text_color(cx.theme().link)
                        .child("Documentation ↗"),
                );
            if let Some(entry) = self.catalog.iter().find(|entry| entry.key == package.key) {
                details = details
                    .child(entry.when_useful.clone())
                    .child(entry.context.clone())
                    .child(entry.caveats.clone())
                    .child(
                        "Example uses catalog aliases; adapt it to the actual project declaration.",
                    )
                    .child(
                        div()
                            .id("composition-package-example")
                            .overflow_x_scroll()
                            .font_family("Consolas")
                            .text_sm()
                            .child(entry.example.clone()),
                    );
            }
        } else {
            details = details.child("Select Details to read package guidance and documentation.");
        }
        details = details.child(div().font_weight(FontWeight::SEMIBOLD).child("Actual Wally declarations · read only"))
            .child("Manual declarations are not imported into the recorded composition. Declared pins, resolved versions and installed files are separate facts.");
        if let Some(snapshot) = &self.composition.snapshot {
            if snapshot.packages.is_empty() {
                details =
                    details.child("No declared Wally dependencies in this captured snapshot.");
            }
            if !snapshot.packages.is_empty() {
                details = details.child(
                    uniform_list(
                        "composition-declarations",
                        snapshot.packages.len(),
                        cx.processor(|this, range: std::ops::Range<usize>, _, _| {
                            range
                                .filter_map(|index| {
                                    this.composition
                                        .snapshot
                                        .as_ref()
                                        .and_then(|snapshot| snapshot.packages.get(index))
                                })
                                .map(|package| {
                                    div()
                                        .id(SharedString::from(format!(
                                            "composition-declared-{}:{}",
                                            package.realm.section(),
                                            package.alias
                                        )))
                                        .role(Role::Group)
                                        .aria_label(format!(
                                            "{}: {}",
                                            package.alias, package.declaration
                                        ))
                                        .test_support()
                                        .v_flex()
                                        .gap_1()
                                        .h(rems(7.))
                                        .min_w_0()
                                        .overflow_x_scroll()
                                        .whitespace_nowrap()
                                        .text_sm()
                                        .child(format!(
                                            "{} · {}",
                                            package.alias,
                                            package.realm.section()
                                        ))
                                        .child(format!("Declared: {}", package.declaration))
                                        .child(format!(
                                            "Resolved: {} · {:?}",
                                            package
                                                .resolved
                                                .as_ref()
                                                .map(|r| r.version.as_str())
                                                .unwrap_or("Not known"),
                                            package.resolution
                                        ))
                                        .child(format!(
                                            "Installed files: {:?}",
                                            package.presence.state
                                        ))
                                })
                                .collect::<Vec<_>>()
                        }),
                    )
                    .h(rems(12.))
                    .w_full(),
                );
            }
        }
        div()
            .h_flex()
            .items_stretch()
            .gap_4()
            .when(stacked, |view| view.flex_col())
            .child(list)
            .child(details)
            .into_any_element()
    }
    fn composition_capabilities(&self, cx: &mut Context<Self>) -> AnyElement {
        let Some(draft) = &self.composition.draft else {
            return div()
                .child("Reload a supported recorded composition first")
                .into_any_element();
        };
        let mut body = div().v_flex().gap_4();
        let busy = self.composition_running() || self.composition.result.is_some();
        for (key, value) in &draft.original.capabilities {
            if capabilities::find(key).is_none() {
                body = body.child(format!(
                    "Recorded / locked: {key}: {value} · Unsupported; Apply is blocked"
                ));
            }
        }
        for capability in capabilities::CAPABILITIES {
            let key = capability.key;
            let selected = draft.graph.capabilities.get(key);
            let locked = draft.original.capabilities.contains_key(key);
            let missing =
                capability.missing_requirements(|key| draft.graph.capabilities.contains_key(key));
            body = body.child(div().v_flex().gap_2().pb_3().border_b_1().border_color(cx.theme().border)
                .child(div().font_weight(FontWeight::SEMIBOLD).child(capability.outcome))
                .when(locked, |view| view.child(format!("Recorded / locked: {}", selected.map(String::as_str).unwrap_or("Unsupported"))))
                .child(div().h_flex().flex_wrap().gap_2()
                    .child(Button::new(SharedString::from(format!("composition-cap-{key}-none"))).label("None").selected(selected.is_none()).disabled(busy || locked).on_click(cx.listener(move |this, _, _, cx| this.mutate_composition(|draft| draft.set_capability(key, None), cx))))
                    .children(capability.implementations.iter().map(|implementation| {
                        let value = implementation.key;
                        Button::new(SharedString::from(format!("composition-cap-{key}-{value}"))).label(if value == "jest-roblox" { "Jest Roblox (Studio)" } else { implementation.display })
                            .selected(selected.is_some_and(|s| s == value)).disabled(busy || locked || !implementation.workflows.contains(&draft.graph.package_workflow) || !missing.is_empty())
                            .on_click(cx.listener(move |this, _, _, cx| this.mutate_composition(|draft| draft.set_capability(key, Some(value)), cx)))
                    })))
                .when(!missing.is_empty(), |view| view.child(format!("Requires: {}. Select these capabilities first.", missing.join(", "))))
                .when(key == "test", |view| view.child("Jest Roblox requires Wally. Studio needs its existing runner plugin; Open Cloud needs credentials configured outside this editor. TestEZ also supports None. No runner or credential installer is included here."))
                .when(key == "assets-3d", |view| view.child("Blender must already be installed. Review checks it before any project writes; only a newly requested missing scene is generated.")));
        }
        body.child(format!(
            "Preserved exclusions: {}",
            if draft.original.dropped.is_empty() {
                "None".into()
            } else {
                draft.original.dropped.join(", ")
            }
        ))
        .into_any_element()
    }
    fn composition_review(&self, cx: &mut Context<Self>) -> AnyElement {
        let mut body = div().v_flex().gap_3();
        if let Some(result) = &self.composition.result {
            return match result {
                Ok(result) => body.child(format!("{} · {}", outcome_label(result.status), result.project.display()))
                    .child(format!("{}. {}", writes_label(result.writes), result.message.as_deref().unwrap_or("")))
                    .when(result.status != CompositionStatus::Succeeded, |view| view.child("Inspect the captured project for possible partial changes. No rollback is promised. Reload explicitly before retrying."))
                    .when(result.status == CompositionStatus::Succeeded, |view| view.child(Button::new("composition-open-result").label("Open project").on_click(cx.listener(|this, _, _, cx| {
                        if let Some(Ok(result)) = &this.composition.result && result.status == CompositionStatus::Succeeded { this.open_project(result.project.clone(), cx); }
                    }))))
                    .into_any_element(),
                Err(error) => body.child(format!("Composition outcome / teardown unconfirmed: {error}"))
                    .child("Inspect Activity and the captured project for possible partial changes. Success was not confirmed.").into_any_element(),
            };
        }
        body = body.child("Impact review — estimated effects, not exact file diffs. Existing declarations, tool pins and exclusions stay intact. Generated configuration may refresh; the composition record may be reformatted and lose comments. Downloads and resolution/generated output can change. No project-wide rollback is promised.");
        if self.composition.watch_conflict(self.watch.as_ref()) {
            body = body.child(div().id("composition-watch-conflict").role(Role::Alert).aria_label("Watch blocks Apply on this project").test_support().child("Watch is active or unconfirmed on this captured project. Use the separate global Stop control and wait for Stopped before Apply. Browsing or Apply never switches Watch."));
        }
        if let Some(draft) = &self.composition.draft
            && let Ok(additions) = draft.additions()
        {
            body = body
                .child(format!(
                    "Added packages: {}",
                    additions
                        .packages
                        .iter()
                        .filter(|k| !capability_owned(k))
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(", ")
                ))
                .child(format!(
                    "Derived packages: {}",
                    additions
                        .packages
                        .iter()
                        .filter(|k| capability_owned(k))
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(", ")
                ))
                .child(format!(
                    "Added capabilities: {}",
                    additions
                        .capabilities
                        .iter()
                        .map(|(k, v)| format!("{k}: {v}"))
                        .collect::<Vec<_>>()
                        .join(", ")
                ))
                .child(format!(
                    "Preserved file exclusions: {}",
                    draft.original.dropped.join(", ")
                ))
                .when(!self.composition.dirty, |view| {
                    view.child(
                        "No additions selected. Apply is unavailable until something is added.",
                    )
                });
        }
        if let Some(plan) = self.composition.fresh_plan() {
            body = body.child(
                div()
                    .id("composition-reviewed-impact")
                    .role(Role::Group)
                    .aria_label("Prepared composition impact")
                    .test_support()
                    .v_flex()
                    .gap_2()
                    .child(format!(
                        "Missing tool pins to add: {}",
                        plan.tools_to_add.join(", ")
                    ))
                    .children(plan.files.iter().map(|file| {
                        div()
                            .v_flex()
                            .gap_1()
                            .child(format!(
                                "{} {}",
                                if file.creating { "Create" } else { "Refresh" },
                                file.relative
                            ))
                            .child(file.reason.clone())
                    }))
                    .children(
                        plan.external_operations
                            .iter()
                            .map(|operation| div().child(operation.clone())),
                    ),
            );
        } else {
            body = body.child("Prepare Review off the UI thread. Changed source files invalidate the plan and require explicit Reload; no automatic merge is performed.");
        }
        body.into_any_element()
    }
}
