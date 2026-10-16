use super::*;
use gpui_kit::component::{
    button::*,
    checkbox::Checkbox,
    input::Input,
    menu::{DropdownMenu, PopupMenuItem},
};
use gpui_kit::prelude::FluentBuilder;
use rproj_core::{
    catalog::{
        artifacts, capabilities,
        wally_packages::{self, Category},
    },
    creation::{PackageWorkflow, offerable_package},
};

impl Workbench {
    pub(in super::super) fn render_creation(&self, cx: &mut Context<Self>) -> AnyElement {
        let busy = self.running.is_some() || self.creation.teardown_uncertain;
        let finished = self.creation.result.is_some();
        let step = self.creation.step;
        let body = match step {
            Step::Project => self.creation_project(cx),
            Step::Dependencies => self.creation_dependencies(cx),
            Step::Capabilities => self.creation_capabilities(cx),
            Step::Files => self.creation_files(cx),
            Step::Review => self.creation_review(cx),
        };
        div()
            .id("creation-flow")
            .role(Role::Group)
            .aria_label(format!("New project: {}", step.title()))
            .test_support()
            .track_focus(&self.creation_inputs.focus)
            .v_flex()
            .size_full()
            .min_h_0()
            .child(div().h_flex().flex_wrap().gap_2().px_6().py_2().children(
                Step::ALL.into_iter().map(|entry| {
                    div()
                        .text_sm()
                        .text_color(if step == entry {
                            cx.theme().foreground
                        } else {
                            cx.theme().muted_foreground
                        })
                        .child(format!("{}. {}", entry.index() + 1, entry.title()))
                }),
            ))
            .child(
                div()
                    .id("creation-scroll")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .child(
                        div()
                            .v_flex()
                            .gap_4()
                            .p_6()
                            .when_some(
                                self.creation
                                    .error
                                    .clone()
                                    .or_else(|| self.creation.selection_problem.clone()),
                                |view, error| {
                                    view.child(
                                        div()
                                            .id("creation-error")
                                            .role(Role::Alert)
                                            .aria_label(error.clone())
                                            .test_support()
                                            .text_color(cx.theme().danger)
                                            .child(error),
                                    )
                                },
                            )
                            .child(body),
                    ),
            )
            .child(
                div()
                    .h_flex()
                    .flex_none()
                    .flex_wrap()
                    .gap_2()
                    .px_6()
                    .py_3()
                    .border_t_1()
                    .border_color(cx.theme().border)
                    .child(
                        Button::new("creation-discard")
                            .ghost()
                            .label(if finished {
                                "Finish"
                            } else {
                                "Discard draft…"
                            })
                            .disabled(busy)
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.discard_creation(false, window, cx)
                            })),
                    )
                    .child(div().flex_1())
                    .child(
                        Button::new("creation-back")
                            .label("Back")
                            .disabled(busy || step == Step::Project || finished)
                            .on_click(cx.listener(|this, _, window, cx| {
                                if this.creation.step != Step::Project && this.running.is_none() {
                                    this.creation.step = Step::ALL[this.creation.step.index() - 1];
                                    this.focus_creation(window, cx);
                                    cx.notify();
                                }
                            })),
                    )
                    .child(if step == Step::Review {
                        div()
                            .h_flex()
                            .gap_2()
                            .child(
                                Button::new("creation-prepare")
                                    .label("Prepare review")
                                    .disabled(busy || finished || self.creation.loading)
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.creation_next(window, cx)
                                    })),
                            )
                            .child(
                                Button::new("creation-create")
                                    .primary()
                                    .label("Create project…")
                                    .disabled(
                                        busy || finished || self.creation.fresh_plan().is_none(),
                                    )
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.confirm_creation(window, cx)
                                    })),
                            )
                            .into_any_element()
                    } else {
                        Button::new("creation-next")
                            .primary()
                            .label("Next")
                            .disabled(
                                busy || finished
                                    || self.creation.context.is_none()
                                    || self.creation.loading,
                            )
                            .on_click(
                                cx.listener(|this, _, window, cx| this.creation_next(window, cx)),
                            )
                            .into_any_element()
                    }),
            )
            .into_any_element()
    }
    fn creation_project(&self, cx: &mut Context<Self>) -> AnyElement {
        div().v_flex().gap_3()
            .child(div().text_lg().font_weight(FontWeight::SEMIBOLD).child("Name and destination"))
            .child("Project name")
            .child(Input::new(&self.creation_inputs.name).id("creation-name").aria_label("Project name").disabled(self.creation_running() || self.creation.result.is_some()))
            .child(div().id("creation-destination").role(Role::Group).aria_label(self.creation.destination().map(|p| p.display().to_string()).unwrap_or_else(|| "Destination unavailable".into())).test_support()
                .min_w_0().overflow_x_scroll().whitespace_nowrap().text_sm().child(self.creation.destination().map(|p| format!("Destination: {}", p.display())).unwrap_or_else(|| "Destination: waiting for configured project root".into())))
            .child(self.creation.context.as_ref().map(|context| {
                if !context.machine_ready { "Machine setup is missing. Run rproj setup in a terminal, then Recheck.".into() }
                else if let Some(problem) = &context.template_problem { format!("Template needs attention: {problem}. Use the existing terminal configuration workflow, then Recheck.") }
                else if context.template.is_some() { "Using your configured Rojo template. Preparation validates it without creating the destination.".into() }
                else { "Using the standard Rojo project template.".into() }
            }).unwrap_or_else(|| if self.creation.loading { "Loading creation context…".into() } else { "New Project needs a bundled CLI with creation support. Existing workbench actions remain available.".into() }))
            .child(Button::new("creation-recheck").label(if self.creation.loading { "Rechecking…" } else { "Recheck" }).disabled(self.creation.loading || self.creation_running() || self.client.is_none())
                .on_click(cx.listener(|this, _, _, cx| this.load_creation_context(cx))))
            .child("Opening or creating a project never changes the active Watch.")
            .into_any_element()
    }
    fn creation_dependencies(&self, cx: &mut Context<Self>) -> AnyElement {
        let Some(draft) = &self.creation.draft else {
            return div().into_any_element();
        };
        let disabled = self.creation_running() || self.creation.result.is_some();
        let guided = draft.graph.mode == "guided";
        let owner = cx.entity().downgrade();
        let setups = self
            .creation
            .context
            .as_ref()
            .map(|c| c.setups.clone())
            .unwrap_or_default();
        let query = self.creation_inputs.query.read(cx).value().to_lowercase();
        let category = self.creation.category;
        let mut body = div().v_flex().gap_4()
            .child("Starting point")
            .child(div().h_flex().gap_2().flex_wrap()
                .children([("guided", "Guided"), ("expert", "Expert")].into_iter().map(|(mode, label)|
                    Button::new(SharedString::from(format!("creation-{mode}"))).label(label).selected(draft.graph.mode == mode && self.creation.selected_setup.is_none()).disabled(disabled)
                        .on_click(cx.listener(move |this, _, _, cx| {
                            if !this.creation_running() && this.creation.result.is_none() {
                                if let Err(error) = this.creation.select_mode(mode) { this.creation.error = Some(error); }
                                cx.notify();
                            }
                        }))))
                .child(Button::new("creation-saved").label(self.creation.selected_setup.clone().unwrap_or_else(|| "Saved setup…".into())).disabled(disabled || setups.is_empty())
                    .dropdown_menu(move |mut menu, _, _| {
                        for setup in &setups {
                            let name = setup.name.clone(); let owner = owner.clone();
                            menu = menu.item(PopupMenuItem::new(name.clone()).on_click(move |_, _, cx| {
                                let _ = owner.update(cx, |this, cx| {
                                    if !this.creation_running() { if let Err(error) = this.creation.load_setup(&name) { this.creation.error = Some(error); } cx.notify(); }
                                });
                            }));
                        } menu
                    })))
            .child("Guided adds companions and keeps one primary choice per category. Switching to Guided recomputes companions and affected file choices; resolve extra primary choices in Expert first. Loading a saved setup replaces composition choices; names remain yours.")
            .child(div().h_flex().gap_2().children([(PackageWorkflow::Wally,"Wally"), (PackageWorkflow::None,"None")].into_iter().map(|(workflow,label)|
                Button::new(SharedString::from(format!("creation-workflow-{label}"))).label(label).selected(draft.graph.package_workflow == workflow).disabled(disabled)
                    .on_click(cx.listener(move |this, _, window, cx| this.confirm_workflow(workflow, window, cx))))))
            .child(format!("Selected packages (including derived companions): {}", if draft.graph.packages.is_empty() { "None".into() } else { draft.graph.packages.join(", ") }))
            .child(Input::new(&self.creation_inputs.query).id("creation-package-search").aria_label("Search packages"))
            .child(div().h_flex().flex_wrap().gap_2().children(Category::ALL.into_iter().filter(|c| *c != Category::Testing).map(|category|
                Button::new(SharedString::from(format!("creation-category-{}", category.label()))).ghost().label(category.label()).selected(self.creation.category == category)
                    .on_click(cx.listener(move |this, _, _, cx| { this.creation.category = category; cx.notify(); })))))
            .when(draft.graph.package_workflow == PackageWorkflow::None, |v| v.child("Package choices require Wally. TestEZ is configured separately in Capabilities."));
        let packages: Vec<_> = wally_packages::PACKAGES
            .iter()
            .filter(|p| {
                p.category == category
                    && offerable_package(p, draft.graph.package_workflow)
                    && (!guided || p.primary_choice)
                    && (query.is_empty()
                        || format!("{} {} {}", p.key, p.module_name, p.description)
                            .to_lowercase()
                            .contains(&query))
            })
            .collect();
        if packages.is_empty() {
            body = body.child("No package choices match this category, workflow and search.");
        }
        for package in packages {
            let key = package.key;
            body = body.child(
                div()
                    .v_flex()
                    .gap_2()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .pb_3()
                    .child(
                        Checkbox::new(SharedString::from(format!("creation-package-{key}")))
                            .label(package.module_name)
                            .checked(draft.graph.packages.iter().any(|p| p == key))
                            .disabled(disabled)
                            .on_change(cx.listener(move |this, _, _, cx| {
                                this.mutate_creation(|draft| draft.toggle_package(key, guided), cx)
                            })),
                    )
                    .child(package.description)
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(format!(
                                "{} · {}",
                                package.source,
                                package.maintenance.badge()
                            )),
                    )
                    .child(
                        Button::new(SharedString::from(format!("creation-detail-{key}")))
                            .ghost()
                            .label("Package details")
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.creation.selected_package = Some(key.into());
                                cx.notify();
                            })),
                    ),
            );
        }
        if let Some(package) = self
            .creation
            .selected_package
            .as_deref()
            .and_then(wally_packages::find)
        {
            let detail = self.catalog.iter().find(|entry| entry.key == package.key);
            body =
                body.child(
                    div()
                        .id("creation-package-detail")
                        .v_flex()
                        .gap_3()
                        .child(
                            div()
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(package.module_name),
                        )
                        .child(package.description)
                        .child(
                            gpui_kit::base::Link::new("creation-package-docs")
                                .href(package.docs_url)
                                .text_color(cx.theme().link)
                                .child("Documentation ↗"),
                        )
                        .when_some(detail, |view, entry| {
                            view.child(entry.when_useful.clone())
                                .child(entry.context.clone())
                                .child(entry.caveats.clone())
                                .child(div().text_sm().child(
                                    "Example uses catalog aliases; adapt them to your project.",
                                ))
                                .child(
                                    div()
                                        .id("creation-example")
                                        .overflow_x_scroll()
                                        .font_family("Consolas")
                                        .text_sm()
                                        .child(entry.example.clone()),
                                )
                        }),
                );
        }
        body.into_any_element()
    }
    fn creation_capabilities(&self, cx: &mut Context<Self>) -> AnyElement {
        let Some(draft) = &self.creation.draft else {
            return div().into_any_element();
        };
        let mut body = div().v_flex().gap_4().child("Choose the outcomes you need. Testing is optional; choose its concrete runner and backend explicitly.");
        for capability in capabilities::CAPABILITIES {
            let key = capability.key;
            let selected = draft.graph.capabilities.get(key);
            let missing =
                capability.missing_requirements(|key| draft.graph.capabilities.contains_key(key));
            let disabled = self.creation_running() || self.creation.result.is_some();
            body = body.child(div().v_flex().gap_2().pb_3().border_b_1().border_color(cx.theme().border)
                .child(div().font_weight(FontWeight::SEMIBOLD).child(capability.outcome))
                .child(div().h_flex().flex_wrap().gap_2()
                    .child(Button::new(SharedString::from(format!("creation-cap-{key}-none"))).label("None").selected(selected.is_none()).disabled(disabled)
                        .on_click(cx.listener(move |this, _, _, cx| this.mutate_creation(|draft| draft.set_capability(key, None), cx))))
                    .children(capability.implementations.iter().map(|implementation| {
                        let compatible = implementation.workflows.contains(&draft.graph.package_workflow) && missing.is_empty();
                        let value = implementation.key;
                        Button::new(SharedString::from(format!("creation-cap-{key}-{value}"))).label(if value == "jest-roblox" { "Jest Roblox (Studio)" } else { implementation.display }).selected(selected.is_some_and(|s| s == value)).disabled(disabled || !compatible)
                            .on_click(cx.listener(move |this, _, _, cx| this.mutate_creation(|draft| draft.set_capability(key, Some(value)), cx)))
                    })))
                .when(!missing.is_empty(), |v| v.child(format!("Requires: {}. Select these capabilities first.", missing.join(", "))))
                .when(key == "test", |v| v.child("Jest Roblox requires Wally. Studio uses the Studio runner plugin; Open Cloud needs existing credentials configured outside this flow. TestEZ also works without a dependency manager."))
                .when(key == "assets-3d", |v| v.child("Blender must already be installed; execution checks it before claiming the destination."))
                .when_some(selected.filter(|value| capability.implementation(value).is_none_or(|i| !i.workflows.contains(&draft.graph.package_workflow))), |v, value| v.child(format!("Incompatible selection: {value}. Choose a compatible implementation or None explicitly."))));
        }
        body.into_any_element()
    }
    fn creation_files(&self, cx: &mut Context<Self>) -> AnyElement {
        let Some(draft) = &self.creation.draft else {
            return div().into_any_element();
        };
        let Some(context) = &self.creation.context else {
            return div().child("Recheck creation context.").into_any_element();
        };
        let environment = context.environment.clone();
        let plan = draft.plan(&environment);
        let mut body = div().v_flex().gap_3().child("Planned file groups and reasons. Required entries stay included; upstream changes reset optional exclusions.");
        match plan {
            Err(error) => body = body.child(error.to_string()),
            Ok(plan) => {
                for file in plan.files {
                    let key = file.key.clone();
                    let environment = environment.clone();
                    let label = if key == "blender" {
                        "blender/scene.blend"
                    } else {
                        &key
                    };
                    body = body.child(
                        div()
                            .v_flex()
                            .gap_1()
                            .child(
                                Checkbox::new(SharedString::from(format!("creation-file-{key}")))
                                    .label(format!(
                                        "{label}{}",
                                        if file.required { " · Required" } else { "" }
                                    ))
                                    .checked(file.included)
                                    .disabled(
                                        file.required
                                            || self.creation_running()
                                            || self.creation.result.is_some(),
                                    )
                                    .on_change(cx.listener(move |this, _, _, cx| {
                                        this.mutate_creation(
                                            |draft| draft.toggle_file(&key, &environment),
                                            cx,
                                        )
                                    })),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(format!(
                                        "{} — {}",
                                        artifacts::find(&file.key)
                                            .map(|a| a.description)
                                            .unwrap_or("File group"),
                                        file.reason
                                    )),
                            ),
                    );
                }
            }
        }
        body.into_any_element()
    }
    fn creation_review(&self, cx: &mut Context<Self>) -> AnyElement {
        let mut body = div().v_flex().gap_3();
        if let Some(result) = &self.creation.result {
            return match result {
                Ok(result) => body.child(format!("Creation: {:?} · {}", result.status, result.destination.display()))
                    .child(format!("Destination claim: {:?}. {}", result.claim, result.message.as_deref().unwrap_or("")))
                    .when(result.status != CreationStatus::Succeeded, |v| v.child("Partial files may remain. Inspect this destination before retrying; choose a new name for another project."))
                    .when(result.status == CreationStatus::Succeeded, |v| v.child(Button::new("creation-open-result").label("Open project").on_click(cx.listener(|this, _, _, cx| {
                        if let Some(Ok(result)) = &this.creation.result && result.status == CreationStatus::Succeeded { this.open_project(result.destination.clone(), cx); }
                    }))))
                    .into_any_element(),
                Err(error) => body.child(format!("Creation outcome is uncertain: {error}"))
                    .child(format!("Inspect {} for partial files. Success was not confirmed. The workbench has kept this result in Activity.", self.creation.destination().map(|p| p.display().to_string()).unwrap_or_default())).into_any_element(),
            };
        }
        body = body.child("Save this composition as a new named setup (optional)")
            .child(Input::new(&self.creation_inputs.setup).id("creation-setup-name").aria_label("New setup name").disabled(self.creation_running()))
            .child("Changing the name or any choice requires preparing the review again. Existing named setups are never overwritten.");
        if let Some(draft) = &self.creation.draft {
            body = body
                .child(format!(
                    "Workflow: {:?} · Starting point: {}",
                    draft.graph.package_workflow,
                    self.creation
                        .selected_setup
                        .as_deref()
                        .unwrap_or(&draft.graph.mode)
                ))
                .child(format!(
                    "Capabilities: {}",
                    draft
                        .graph
                        .capabilities
                        .iter()
                        .map(|(k, v)| format!("{k}: {v}"))
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            if let Some(context) = &self.creation.context
                && let Ok(plan) = draft.plan(&context.environment)
            {
                body = body
                    .child(
                        div()
                            .id("review-destination")
                            .min_w_0()
                            .overflow_x_scroll()
                            .whitespace_nowrap()
                            .child(format!("Destination: {}", plan.destination.display())),
                    )
                    .child(format!(
                        "Packages: {}",
                        if plan.packages.is_empty() {
                            "None".into()
                        } else {
                            plan.packages.join(", ")
                        }
                    ))
                    .child(format!(
                        "Tool pins: {}",
                        if plan.tools.is_empty() {
                            "None".into()
                        } else {
                            plan.tools.join(", ")
                        }
                    ))
                    .child(format!(
                        "Included file groups: {}",
                        plan.files
                            .iter()
                            .filter(|f| f.included)
                            .map(|f| f.key.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    ))
                    .child(format!(
                        "Excluded optional groups: {}",
                        plan.files
                            .iter()
                            .filter(|f| !f.included)
                            .map(|f| f.key.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    ));
            }
            if draft
                .graph
                .capabilities
                .get("test")
                .is_some_and(|v| v == "jest-roblox")
            {
                body = body.child("Studio runner: creation may install or refresh the Jest Studio runner plugin and update its recorded revision in rproj configuration.");
            }
        }
        body.child("Creation initializes Git, writes the scaffold and project records, installs/resolves selected tools and dependencies, and generates their configured artifacts. These operations may access the network. No Watch is started or switched.")
            .child(if self.creation_running() { "Working in the background. Activity contains human tool output; you can browse existing projects." } else if self.creation.fresh_plan().is_some() { "Review is prepared. Create project asks for final confirmation and revalidates the captured sources and destination." } else { "Prepare review to validate current configuration, template, setup identity and destination. This creates no destination. External tools can still report missing prerequisites during creation." })
            .into_any_element()
    }
}
