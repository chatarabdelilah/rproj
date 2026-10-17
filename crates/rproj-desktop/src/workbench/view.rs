use super::*;
use gpui_kit::component::{
    button::*,
    input::Input,
    menu::{DropdownMenu, PopupMenuItem},
    resizable::{resizable_panel, v_resizable},
    tooltip::Tooltip,
};
use gpui_kit::prelude::FluentBuilder;
use rproj_core::{PackageDependency, PresenceState, ResolutionState, operations::WatchState};

impl Workbench {
    fn render_sidebar(&self, cx: &mut Context<Self>) -> AnyElement {
        let recent = self.preferences.recent_projects.clone();
        let owner = cx.entity().downgrade();
        let has_project = self.project.is_some();
        let has_draft = self.creation.draft.is_some();
        let creation_unavailable = self.creation.context.is_none();
        let creation_reason = self.creation.error.clone();
        let creation_loading = self.creation.loading;
        let composition_name = self
            .composition
            .context
            .as_ref()
            .map(|c| c.name.clone())
            .unwrap_or_else(|| "captured project".into());
        let has_composition = self.composition.project.is_some();
        let can_recheck = self.client.is_some() && !self.creation_running();
        let project_name = self
            .snapshot
            .as_ref()
            .and_then(|snapshot| snapshot.name.clone())
            .or_else(|| {
                self.project
                    .as_ref()
                    .and_then(|path| path.file_name())
                    .map(|name| name.to_string_lossy().into_owned())
            })
            .unwrap_or_else(|| "Open project…".into());
        let switcher = Button::new("project-switcher")
            .w_full()
            .label(project_name)
            .icon(IconName::FolderOpen)
            .dropdown_menu(move |mut menu, _, _| {
                if recent.is_empty() {
                    menu = menu.label("No recent projects");
                }
                for path in &recent {
                    let name = path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into_owned();
                    let display = path.display().to_string();
                    let path = path.clone();
                    let owner = owner.clone();
                    menu = menu.item(
                        PopupMenuItem::element(move |_, cx| {
                            div()
                                .id(SharedString::from(display.clone()))
                                .role(Role::Group)
                                .aria_label(format!("{name}: {display}"))
                                .v_flex()
                                .gap_1()
                                .max_w(rems(26.))
                                .child(name.clone())
                                .child(
                                    div()
                                        .text_sm()
                                        .text_color(cx.theme().muted_foreground)
                                        .child(display.clone()),
                                )
                        })
                        .on_click(move |_, _, cx| {
                            let _ =
                                owner.update(cx, |this, cx| this.open_project(path.clone(), cx));
                        }),
                    );
                }
                let open = owner.clone();
                let close = owner.clone();
                let create = owner.clone();
                let recheck = owner.clone();
                let composition = owner.clone();
                if creation_unavailable {
                    menu =
                        menu.label(creation_reason.clone().unwrap_or_else(|| {
                            "New Project: connecting to the bundled CLI…".into()
                        }));
                }
                menu.separator()
                    .item(
                        PopupMenuItem::new(if has_draft {
                            "Resume project creation"
                        } else {
                            "New project…"
                        })
                        .icon(IconName::Plus)
                        .disabled(creation_unavailable && !has_draft)
                        .on_click(move |_, window, cx| {
                            let _ = create.update(cx, |this, cx| this.show_creation(window, cx));
                        }),
                    )
                    .item(
                        PopupMenuItem::new(format!("Resume editing {composition_name}"))
                            .disabled(!has_composition)
                            .on_click(move |_, window, cx| {
                                let _ = composition
                                    .update(cx, |this, cx| this.show_composition(window, cx));
                            }),
                    )
                    .item(
                        PopupMenuItem::new("Recheck creation availability")
                            .disabled(creation_loading || !can_recheck)
                            .on_click(move |_, _, cx| {
                                let _ =
                                    recheck.update(cx, |this, cx| this.load_creation_context(cx));
                            }),
                    )
                    .item(
                        PopupMenuItem::new("Open project…")
                            .icon(IconName::FolderOpen)
                            .on_click(move |_, _, cx| {
                                let _ = open.update(cx, |this, cx| this.choose_project(cx));
                            }),
                    )
                    .item(
                        PopupMenuItem::new("Close project")
                            .disabled(!has_project)
                            .on_click(move |_, _, cx| {
                                let _ = close.update(cx, |this, cx| this.close_project(cx));
                            }),
                    )
            });
        let appearance = self.preferences.appearance;
        div()
            .v_flex()
            .w(theme::SIDEBAR_WIDTH)
            .h_full()
            .flex_none()
            .border_r_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().sidebar)
            .p(theme::PANEL_PADDING)
            .gap(theme::GROUP_GAP)
            .child(
                div()
                    .text_xl()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("rproj"),
            )
            .child(switcher)
            .child(
                div().v_flex().flex_1().gap_2().children(
                    [
                        Section::Overview,
                        Section::Dependencies,
                        Section::Tools,
                        Section::Checks,
                    ]
                    .into_iter()
                    .map(|section| {
                        Button::new(section.title())
                            .ghost()
                            .w_full()
                            .label(section.title())
                            .icon(section.icon())
                            .selected(
                                self.section == section
                                    && !self.creation.visible
                                    && !self.composition.visible,
                            )
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.section = section;
                                this.creation.visible = false;
                                this.composition.visible = false;
                                cx.notify();
                            }))
                    }),
                ),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child("Appearance"),
            )
            .children(
                [Appearance::System, Appearance::Light, Appearance::Dark]
                    .into_iter()
                    .map(|choice| {
                        let label = match choice {
                            Appearance::System => "System",
                            Appearance::Light => "Light",
                            Appearance::Dark => "Dark",
                        };
                        Button::new(label)
                            .ghost()
                            .label(label)
                            .selected(appearance == choice)
                            .disabled(self.preferences_path.is_none())
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.preferences.appearance = choice;
                                apply_appearance(choice, window, cx);
                                this.save_preferences();
                                cx.notify();
                            }))
                    }),
            )
            .when_some(self.preferences_error.clone(), |view, error| {
                view.child(div().text_xs().child(error))
            })
            .into_any_element()
    }

    fn render_overview(&self, cx: &mut Context<Self>) -> AnyElement {
        let Some(snapshot) = &self.snapshot else {
            return div()
                .v_flex()
                .gap_3()
                .p_6()
                .child(div().text_xl().child(if self.inspecting {
                    "Reading project…"
                } else if self.error.is_some() {
                    "Project could not be read"
                } else {
                    "Open your project"
                }))
                .child(
                    "Choose an existing rproj or Rojo project folder. Its files stay in control.",
                )
                .child(
                    Button::new("new-project")
                        .label(if self.creation.draft.is_some() {
                            "Resume project creation"
                        } else {
                            "New project…"
                        })
                        .disabled(self.creation.context.is_none() && self.creation.draft.is_none())
                        .on_click(
                            cx.listener(|this, _, window, cx| this.show_creation(window, cx)),
                        ),
                )
                .when_some(self.creation.error.clone(), |view, error| view.child(error))
                .when(self.creation.context.is_none(), |view| {
                    view.child("New Project needs a matching bundled CLI with creation support.")
                        .child(
                            Button::new("new-project-recheck")
                                .label("Recheck")
                                .disabled(self.creation.loading || self.client.is_none())
                                .on_click(
                                    cx.listener(|this, _, _, cx| this.load_creation_context(cx)),
                                ),
                        )
                })
                .into_any_element();
        };
        let files = [
            &snapshot.files.rproj,
            &snapshot.files.rojo,
            &snapshot.files.wally,
            &snapshot.files.wally_lock,
            &snapshot.files.rokit,
            &snapshot.files.check_script,
        ];
        div().v_flex().gap_4().p_6()
            .child(div().text_lg().font_weight(FontWeight::SEMIBOLD).child("Project configuration"))
            .child(format!("{} declared packages · {} tool pins", snapshot.packages.len(), snapshot.tools.len()))
            .children(files.into_iter().map(|file| div().h_flex().justify_between().gap_4()
                .child(file.path.file_name().unwrap_or_default().to_string_lossy().into_owned())
                .child(format!("{:?}", file.state))))
            .child(div().text_sm().text_color(cx.theme().muted_foreground)
                .child("Dependencies follow wally.toml and wally.lock. Saved rproj selections remain separate."))
            .children(snapshot.diagnostics.iter().map(|diagnostic| div().v_flex().gap_1()
                .child(format!("{:?}: {}", diagnostic.severity, diagnostic.message))
                .child(div().text_sm().text_color(cx.theme().muted_foreground).child(diagnostic.path.display().to_string()))))
            .into_any_element()
    }

    fn render_dependencies(&self, stacked: bool, cx: &mut Context<Self>) -> AnyElement {
        let Some(snapshot) = &self.snapshot else {
            return self.render_overview(cx);
        };
        if snapshot.packages.is_empty() {
            return div()
                .p_6()
                .child(
                    "No declared Wally dependencies. Check Overview for configuration diagnostics.",
                )
                .into_any_element();
        }
        let selected = self.selected_package.as_ref().and_then(|key| {
            snapshot
                .packages
                .iter()
                .find(|package| package_key(package) == *key)
        });
        let list = uniform_list(
            "dependencies",
            snapshot.packages.len(),
            cx.processor(|this, range: std::ops::Range<usize>, _, cx| {
                range
                    .filter_map(|ix| {
                        this.snapshot
                            .as_ref()
                            .and_then(|snapshot| snapshot.packages.get(ix))
                    })
                    .map(|package| {
                        let key = package_key(package);
                        let selected = this.selected_package.as_deref() == Some(&key);
                        Button::new(SharedString::from(key.clone()))
                            .ghost()
                            .w_full()
                            .h(rems(2.5))
                            .label(format!("{}  ·  {}", package.alias, package.realm.section()))
                            .selected(selected)
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.selected_package = Some(key.clone());
                                cx.notify();
                            }))
                    })
                    .collect::<Vec<_>>()
            }),
        )
        .h_full()
        .flex_1();
        let detail = selected
            .map(|package| self.package_detail(package, cx))
            .unwrap_or_else(|| {
                div()
                    .p_6()
                    .child(
                        "Select a package to inspect its requirement, resolved version, and files.",
                    )
                    .into_any_element()
            });
        div()
            .h_flex()
            .when(stacked, |view| view.flex_col())
            .items_stretch()
            .size_full()
            .min_h_0()
            .child(
                div()
                    .id("package-list")
                    .role(Role::Group)
                    .aria_label("Dependencies")
                    .test_support()
                    .when(!stacked, |view| view.w(rems(16.)).h_full().border_r_1())
                    .when(stacked, |view| {
                        view.w_full().h(rems(7.)).flex_none().border_b_1()
                    })
                    .min_h_0()
                    .border_color(cx.theme().border)
                    .child(list),
            )
            .child(
                div()
                    .id("package-detail")
                    .role(Role::Group)
                    .aria_label("Dependency details")
                    .test_support()
                    .flex_1()
                    .min_w_0()
                    .min_h_0()
                    .overflow_y_scroll()
                    .child(detail),
            )
            .into_any_element()
    }

    fn package_detail(&self, package: &PackageDependency, cx: &mut Context<Self>) -> AnyElement {
        let resolved = package
            .resolved
            .as_ref()
            .map(|resolved| resolved.version.clone())
            .unwrap_or_else(|| "Not known".into());
        let presence = match package.presence.state {
            PresenceState::Present => "Alias module and package index present",
            PresenceState::Missing => "Alias module or package index missing",
            PresenceState::Unknown => "Package file state unknown",
        };
        let resolution = match package.resolution {
            ResolutionState::Resolved => "Resolved",
            ResolutionState::Unlocked => "Not locked",
            ResolutionState::Stale => "Lockfile needs attention",
            ResolutionState::Unknown => "Resolution unknown",
        };
        let catalog = self
            .catalog
            .iter()
            .find(|entry| package.package.as_deref() == Some(&entry.package));
        div().v_flex().gap_4().p_6()
            .child(div().text_xl().font_weight(FontWeight::SEMIBOLD).child(package.alias.clone()))
            .child(format!("Declared: {}", package.declaration))
            .child(format!("Resolved: {resolved} · {resolution}"))
            .child(presence)
            .when_some(catalog, |view, entry| {
                view.child(entry.description.clone()).child(entry.when_useful.clone())
                    .child(entry.context.clone()).child(entry.caveats.clone())
                    .child(gpui_kit::base::Link::new("package-documentation").href(entry.docs_url.clone()).text_color(cx.theme().link).child("Documentation ↗"))
                    .child(div().text_sm().child("Example uses the catalog's default alias; adapt it to your project's alias."))
                    .child(div().font_family("Consolas").text_sm().child(entry.example.clone()))
            }).when(catalog.is_none(), |view| view.child(div().text_sm().text_color(cx.theme().muted_foreground)
                .child("This package has no rproj catalog entry. Its manifest declaration remains available above.")))
            .into_any_element()
    }

    fn render_tools(&self, cx: &mut Context<Self>) -> AnyElement {
        let Some(snapshot) = &self.snapshot else {
            return self.render_overview(cx);
        };
        div().v_flex().gap_4().p_6().child(div().text_lg().child("Project tool pins"))
            .child("Opening a project does not install or update tools. PATH detection does not execute tools.")
            .children(snapshot.tools.iter().map(|tool| div().v_flex().gap_1()
                .child(format!("{} · {}", tool.alias, tool.source))
                .child(div().text_sm().text_color(cx.theme().muted_foreground).child(tool.executable.as_ref()
                    .map(|path| format!("Found on PATH: {}", path.display())).unwrap_or_else(|| "Not found on PATH".into())))))
            .into_any_element()
    }

    fn render_checks(&self, cx: &mut Context<Self>) -> AnyElement {
        let Some(snapshot) = &self.snapshot else {
            return self.render_overview(cx);
        };
        let disabled = self.running.is_some() || self.client.is_none();
        div().v_flex().gap_4().p_6().child(div().text_lg().child("Checks and tests"))
            .child("Run the project's configured tools and inspect their output below.")
            .child(Button::new("run-checks").label("Run checks").disabled(disabled || !snapshot.actions.check.available)
                .on_click(cx.listener(|this, _, _, cx| this.start(Operation::Check, cx))))
            .when_some(snapshot.actions.check.reason.clone(), |view, reason| view.child(reason))
            .child(Button::new("run-tests").label("Run tests").disabled(disabled || !snapshot.actions.test.available)
                .on_click(cx.listener(|this, _, _, cx| this.start(Operation::Test, cx))))
            .when_some(snapshot.actions.test.reason.clone(), |view, reason| view.child(reason))
            .child("Checks execute scripts and tools configured by this project. Run them only for projects you trust.")
            .into_any_element()
    }

    fn render_activity(&self, cx: &mut Context<Self>) -> AnyElement {
        let query = self.filter.read(cx).value().to_lowercase();
        let visible: Arc<Vec<_>> = Arc::new(
            self.activity
                .displayed()
                .filter(|line| query.is_empty() || line.to_lowercase().contains(&query))
                .collect(),
        );
        let empty = visible.is_empty();
        let widest = visible
            .iter()
            .enumerate()
            .max_by_key(|(_, line)| line.len())
            .map(|(ix, _)| ix);
        let list = uniform_list("activity-lines", visible.len(), move |range, _, _| {
            range
                .map(|ix| {
                    div()
                        .h(rems(1.5))
                        .whitespace_nowrap()
                        .text_sm()
                        .child(visible[ix].clone())
                })
                .collect::<Vec<_>>()
        })
        .with_width_from_item(widest)
        .with_horizontal_sizing_behavior(ListHorizontalSizingBehavior::Unconstrained)
        .size_full();
        div()
            .v_flex()
            .size_full()
            .min_h_0()
            .child(
                div()
                    .id("activity-toolbar")
                    .role(Role::Group)
                    .aria_label("Activity controls")
                    .test_support()
                    .h_flex()
                    .h(rems(3.))
                    .flex_none()
                    .gap_3()
                    .px_4()
                    .py_2()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(div().font_weight(FontWeight::SEMIBOLD).child("Activity"))
                    .child(
                        Input::new(&self.filter)
                            .id("output-filter")
                            .aria_label("Search output")
                            .w(rems(18.)),
                    )
                    .child(Button::new("clear-output").ghost().label("Clear").on_click(
                        cx.listener(|this, _, _, cx| {
                            this.activity = Activity::default();
                            cx.notify();
                        }),
                    ))
                    .when(self.activity.omitted > 0, |view| {
                        view.child(format!("{} earlier lines omitted", self.activity.omitted))
                    }),
            )
            .child(
                div()
                    .id("activity-output")
                    .role(Role::Group)
                    .aria_label("Operation output")
                    .test_support()
                    .flex_1()
                    .min_h_0()
                    .px_4()
                    .overflow_hidden()
                    .child(if empty {
                        let label = if self.activity.displayed().next().is_some() {
                            "No search matches."
                        } else if self.running.is_some() {
                            "Waiting for output…"
                        } else {
                            "No output yet."
                        };
                        div()
                            .id("output-empty")
                            .role(Role::Status)
                            .aria_label(label)
                            .test_support()
                            .child(label)
                            .into_any_element()
                    } else {
                        list.into_any_element()
                    }),
            )
            .into_any_element()
    }

    fn render_activity_summary(&self, cx: &mut Context<Self>) -> AnyElement {
        let summary = self.operations.summary();
        div()
            .id("activity-summary-row")
            .role(Role::Group)
            .aria_label("Activity status")
            .test_support()
            .h_flex()
            .h(rems(3.))
            .flex_none()
            .gap_2()
            .px_4()
            .py_2()
            .border_t_1()
            .border_color(cx.theme().border)
            .child(
                Button::new("activity-toggle")
                    .ghost()
                    .icon(if self.operations.expanded {
                        IconName::ChevronDown
                    } else {
                        IconName::ChevronUp
                    })
                    .label(if self.operations.expanded {
                        "Hide Activity"
                    } else {
                        "Show Activity"
                    })
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.operations.expanded = !this.operations.expanded;
                        cx.notify();
                    })),
            )
            .child(
                div()
                    .id("operation-summary")
                    .role(Role::Status)
                    .aria_label(summary.clone())
                    .test_support()
                    .flex_1()
                    .min_w_0()
                    .text_sm()
                    .truncate()
                    .tooltip(move |window, cx| Tooltip::new(summary.clone()).build(window, cx))
                    .child(self.operations.compact_summary()),
            )
            .when_some(self.running.as_ref(), |view, (operation, _)| {
                view.child(
                    Button::new("cancel-operation")
                        .label("Cancel")
                        .disabled(
                            !operation.cancellable()
                                || self
                                    .operations
                                    .latest
                                    .as_ref()
                                    .is_some_and(|run| run.status == RunStatus::Cancelling),
                        )
                        .on_click(cx.listener(|this, _, _, cx| {
                            if let Some((_, cancellation)) = &this.running {
                                cancellation.cancel();
                                this.operations.cancel();
                                cx.notify();
                            }
                        })),
                )
            })
            .into_any_element()
    }

    fn render_watch(&self, cx: &mut Context<Self>) -> AnyElement {
        let busy = self.running.is_some() || self.client.is_none();
        let active = self.watch.as_ref().is_some_and(|status| {
            !matches!(status.state, WatchState::Stopped | WatchState::Failed)
        });
        let same_project = self
            .watch
            .as_ref()
            .and_then(|status| status.project.as_ref())
            == self.project.as_ref();
        let available = self
            .snapshot
            .as_ref()
            .is_some_and(|snapshot| snapshot.actions.watch.available)
            && self.watch.as_ref().is_some_and(|status| status.supported);
        let switchable = self.watch.as_ref().is_some_and(|status| {
            matches!(status.state, WatchState::Watching | WatchState::Preparing)
        });
        let label = self
            .watch
            .as_ref()
            .map(|status| {
                format!(
                    "Watch: {:?}{}",
                    status.state,
                    status
                        .project
                        .as_ref()
                        .map(|path| format!(" — {}", path.display()))
                        .unwrap_or_default()
                )
            })
            .unwrap_or_else(|| "Watch: status unavailable".into());
        div().v_flex().gap_2().px_4().py_2().border_t_1().border_color(cx.theme().border)
            .child(div().h_flex().gap_3().child(div().id("watch-summary").role(Role::Status).aria_label(label.clone()).test_support().flex_1().min_w_0().text_sm().truncate().tooltip(move |window, cx| Tooltip::new(label.clone()).build(window, cx)).child(self.watch.as_ref().map(|status| format!("Watch: {:?}{}", status.state, status.project.as_ref().map(|path| format!(" — {}", path.file_name().unwrap_or(path.as_os_str()).to_string_lossy())).unwrap_or_default())).unwrap_or_else(|| "Watch: status unavailable".into())))
                .child(div().track_focus(&self.watch_focus).child(Button::new("watch-start").label(if active && !same_project { "Switch Watch…" } else { "Start Watch" })
                    .disabled(busy || !available || (active && (same_project || !switchable)))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        if active { this.confirm_watch_switch(window, cx); } else { this.start(Operation::StartWatch, cx); }
                    }))))
                .child(Button::new("watch-stop").label("Stop").disabled(busy || !active)
                    .on_click(cx.listener(|this, _, _, cx| this.start(Operation::StopWatch, cx))))
                .child(Button::new("watch-logs").ghost().label("Watch output")
                    .disabled(self.client.is_none() || self.watch_log_request.pending.is_some() || self.changing_watch())
                    .on_click(cx.listener(|this, _, _, cx| this.request_watch_logs(cx)))))
            .child(div().text_xs().text_color(cx.theme().muted_foreground)
                .child("Watch prepares dependencies and maintains the sourcemap. Closing this window keeps Watch running."))
            .when_some(self.client_error.clone(), |view, error| view.child(div().text_sm().child(error)))
            .into_any_element()
    }
}

impl Render for Workbench {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let title = if self.composition.visible {
            self.composition
                .context
                .as_ref()
                .map(|context| format!("Edit {}", context.name))
                .unwrap_or_else(|| "Edit packages & capabilities".into())
        } else if self.creation.visible {
            "New project".into()
        } else {
            self.snapshot
                .as_ref()
                .and_then(|snapshot| snapshot.name.clone())
                .unwrap_or_else(|| "Project workspace".into())
        };
        let content = if self.composition.visible {
            self.render_composition(
                window.viewport_size().width - theme::SIDEBAR_WIDTH.to_pixels(window.rem_size())
                    < px(600.),
                cx,
            )
        } else if self.creation.visible {
            self.render_creation(cx)
        } else {
            match self.section {
                Section::Overview => self.render_overview(cx),
                Section::Dependencies => self.render_dependencies(
                    window.viewport_size().width
                        - theme::SIDEBAR_WIDTH.to_pixels(window.rem_size())
                        < px(600.),
                    cx,
                ),
                Section::Tools => self.render_tools(cx),
                Section::Checks => self.render_checks(cx),
            }
        };
        let content = if !self.composition.visible
            && !self.creation.visible
            && matches!(self.section, Section::Overview | Section::Dependencies)
        {
            div()
                .v_flex()
                .size_full()
                .min_h_0()
                .child(self.composition_entry(cx))
                .child(div().flex_1().min_h_0().child(content))
                .into_any_element()
        } else {
            content
        };
        let content = if self.composition.visible
            || self.creation.visible
            || self.section == Section::Dependencies
        {
            content
        } else {
            div()
                .id("section-scroll")
                .size_full()
                .overflow_y_scroll()
                .child(content)
                .into_any_element()
        };
        let sidebar = self.render_sidebar(cx);
        let activity = self.render_activity(cx);
        let activity_summary = self.render_activity_summary(cx);
        let watch = self.render_watch(cx);
        let content = if self.operations.expanded {
            v_resizable("project-activity")
                .with_state(&self.activity_panels)
                .on_resize(
                    cx.listener(|this, state: &Entity<resizable::ResizableState>, _, cx| {
                        if let Some(height) = state.read(cx).sizes().get(1) {
                            this.activity_height = *height;
                        }
                    }),
                )
                .child(
                    resizable_panel()
                        .size(px(400.))
                        .size_range(px(140.)..px(2000.))
                        .child(content),
                )
                .child(
                    resizable_panel()
                        .size(self.activity_height)
                        .size_range(px(144.)..px(1000.))
                        .child(
                            div()
                                .v_flex()
                                .size_full()
                                .min_h_0()
                                .child(activity_summary)
                                .child(div().flex_1().min_h_0().child(activity)),
                        ),
                )
                .into_any_element()
        } else {
            div()
                .v_flex()
                .size_full()
                .min_h_0()
                .child(div().flex_1().min_h_0().child(content))
                .child(activity_summary)
                .into_any_element()
        };
        div()
            .track_focus(&self.focus)
            .size_full()
            .v_flex()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .child(
                div()
                    .h_flex()
                    .items_stretch()
                    .flex_1()
                    .min_h_0()
                    .child(sidebar)
                    .child(
                        div()
                            .v_flex()
                            .flex_1()
                            .min_w_0()
                            .min_h_0()
                            .child(
                                div()
                                    .v_flex()
                                    .gap_2()
                                    .px_6()
                                    .py_4()
                                    .child(
                                        div()
                                            .h_flex()
                                            .gap_4()
                                            .child(
                                                div()
                                                    .text_xl()
                                                    .font_weight(FontWeight::SEMIBOLD)
                                                    .flex_1()
                                                    .child(title),
                                            )
                                            .child(
                                                Button::new("refresh")
                                                    .ghost()
                                                    .label("Refresh")
                                                    .disabled(
                                                        self.inspecting
                                                            || self.project.is_none()
                                                            || self.composition.visible,
                                                    )
                                                    .on_click(cx.listener(|this, _, _, cx| {
                                                        this.inspect();
                                                        this.poll_status();
                                                        cx.notify();
                                                    })),
                                            ),
                                    )
                                    .when_some(
                                        self.project.as_ref().filter(|_| {
                                            !self.creation.visible && !self.composition.visible
                                        }),
                                        |view, path| {
                                            view.child(
                                                div()
                                                    .id("project-path")
                                                    .role(Role::Group)
                                                    .aria_label(path.display().to_string())
                                                    .test_support()
                                                    .overflow_x_scroll()
                                                    .whitespace_nowrap()
                                                    .text_sm()
                                                    .text_color(cx.theme().muted_foreground)
                                                    .child(path.display().to_string()),
                                            )
                                        },
                                    )
                                    .when_some(self.error.clone(), |view, error| view.child(error))
                                    .when_some(self.catalog_error.clone(), |view, error| {
                                        view.child(
                                            div()
                                                .id("catalog-error")
                                                .role(Role::Alert)
                                                .aria_label(error.clone())
                                                .child(error)
                                                .test_support(),
                                        )
                                    }),
                            )
                            .child(div().flex_1().min_h_0().child(content)),
                    ),
            )
            .child(watch)
    }
}

fn package_key(package: &PackageDependency) -> String {
    format!("{}:{}", package.realm.section(), package.alias)
}
