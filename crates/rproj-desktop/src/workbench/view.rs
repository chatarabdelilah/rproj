use super::*;
use gpui_kit::component::{
    button::*,
    input::Input,
    resizable::{resizable_panel, v_resizable},
};
use gpui_kit::prelude::FluentBuilder;
use rproj_core::{PackageDependency, PresenceState, ResolutionState, operations::WatchState};

impl Workbench {
    fn render_sidebar(&self, cx: &mut Context<Self>) -> AnyElement {
        let recent = self
            .preferences
            .recent_projects
            .iter()
            .map(|path| {
                let path = path.clone();
                let label = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned();
                Button::new(SharedString::from(format!("recent:{}", path.display())))
                    .ghost()
                    .w_full()
                    .label(label)
                    .tooltip(path.display().to_string())
                    .on_click(
                        cx.listener(move |this, _, _, cx| this.open_project(path.clone(), cx)),
                    )
            })
            .collect::<Vec<_>>();
        let appearance = self.preferences.appearance;
        div()
            .v_flex()
            .w(rems(13.))
            .h_full()
            .flex_none()
            .border_r_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().sidebar)
            .p_4()
            .gap_4()
            .child(
                div()
                    .text_xl()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("rproj"),
            )
            .child(
                Button::new("open-project")
                    .label("Open project…")
                    .icon(IconName::FolderOpen)
                    .on_click(cx.listener(|this, _, _, cx| this.choose_project(cx))),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child("Recent projects"),
            )
            .child(
                div()
                    .id("recent-projects")
                    .v_flex()
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .gap_1()
                    .children(recent),
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
                } else {
                    "Open your project"
                }))
                .child(
                    "Choose an existing rproj or Rojo project folder. Its files stay in control.",
                )
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

    fn render_dependencies(&self, cx: &mut Context<Self>) -> AnyElement {
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
            .items_stretch()
            .size_full()
            .min_h_0()
            .child(
                div()
                    .w(rems(20.))
                    .min_h_0()
                    .border_r_1()
                    .border_color(cx.theme().border)
                    .child(list),
            )
            .child(
                div()
                    .id("package-detail")
                    .flex_1()
                    .min_w_0()
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
        let list = uniform_list("activity-lines", visible.len(), move |range, _, _| {
            range
                .map(|ix| div().h(rems(1.5)).text_sm().child(visible[ix].clone()))
                .collect::<Vec<_>>()
        })
        .size_full();
        div()
            .v_flex()
            .size_full()
            .min_h_0()
            .child(
                div()
                    .h_flex()
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
                    .when_some(self.running.as_ref(), |view, (operation, _)| {
                        view.child(operation_label(*operation)).child(
                            Button::new("cancel-operation")
                                .label("Cancel")
                                .disabled(!operation.cancellable())
                                .on_click(cx.listener(|this, _, _, cx| {
                                    if let Some((_, cancellation)) = &this.running {
                                        cancellation.cancel();
                                        this.activity.append("Cancellation requested…");
                                        cx.notify();
                                    }
                                })),
                        )
                    })
                    .when(self.activity.omitted > 0, |view| {
                        view.child(format!("{} earlier lines omitted", self.activity.omitted))
                    }),
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .px_4()
                    .overflow_hidden()
                    .child(list),
            )
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
            .child(div().h_flex().gap_3().child(div().flex_1().min_w_0().text_sm().child(label))
                .child(Button::new("watch-start").label(if active && !same_project { "Stop and switch" } else { "Start Watch" })
                    .disabled(busy || !available || (active && same_project))
                    .on_click(cx.listener(move |this, _, _, cx| this.start(if active { Operation::SwitchWatch } else { Operation::StartWatch }, cx))))
                .child(Button::new("watch-stop").label("Stop").disabled(busy || !active)
                    .on_click(cx.listener(|this, _, _, cx| this.start(Operation::StopWatch, cx))))
                .child(Button::new("watch-logs").ghost().label("Watch output").disabled(self.client.is_none())
                    .on_click(cx.listener(|this, _, _, _| { if let Some(client) = this.client.clone() {
                        let sender = this.sender.clone(); std::thread::spawn(move || { let _ = sender.send(Event::WatchLogs(client.watch_logs().map_err(|error| format!("{error:#}")))); });
                    } }))))
            .child(div().text_xs().text_color(cx.theme().muted_foreground)
                .child("Watch prepares dependencies and maintains the sourcemap. Closing this window keeps Watch running."))
            .when_some(self.client_error.clone(), |view, error| view.child(div().text_sm().child(error)))
            .into_any_element()
    }
}

impl Render for Workbench {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let title = self
            .snapshot
            .as_ref()
            .and_then(|snapshot| snapshot.name.clone())
            .unwrap_or_else(|| "Project workspace".into());
        let content = match self.section {
            Section::Overview => self.render_overview(cx),
            Section::Dependencies => self.render_dependencies(cx),
            Section::Tools => self.render_tools(cx),
            Section::Checks => self.render_checks(cx),
        };
        let content = if self.section == Section::Dependencies {
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
        let watch = self.render_watch(cx);
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
                                                        self.inspecting || self.project.is_none(),
                                                    )
                                                    .on_click(cx.listener(|this, _, _, cx| {
                                                        this.inspect();
                                                        this.poll_status();
                                                        cx.notify();
                                                    })),
                                            ),
                                    )
                                    .when_some(self.project.as_ref(), |view, path| {
                                        view.child(
                                            div()
                                                .text_sm()
                                                .text_color(cx.theme().muted_foreground)
                                                .child(path.display().to_string()),
                                        )
                                    })
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
                            .child(
                                div()
                                    .h_flex()
                                    .gap_2()
                                    .px_4()
                                    .pb_2()
                                    .border_b_1()
                                    .border_color(cx.theme().border)
                                    .children(
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
                                                .label(section.title())
                                                .selected(self.section == section)
                                                .on_click(cx.listener(move |this, _, _, cx| {
                                                    this.section = section;
                                                    cx.notify();
                                                }))
                                        }),
                                    ),
                            )
                            .child(
                                div().flex_1().min_h_0().child(
                                    v_resizable("project-activity")
                                        .child(
                                            resizable_panel()
                                                .size(px(400.))
                                                .size_range(px(160.)..px(2000.))
                                                .child(content),
                                        )
                                        .child(
                                            resizable_panel()
                                                .size(px(180.))
                                                .size_range(px(110.)..px(1000.))
                                                .child(activity),
                                        ),
                                ),
                            ),
                    ),
            )
            .child(watch)
    }
}

fn package_key(package: &PackageDependency) -> String {
    format!("{}:{}", package.realm.section(), package.alias)
}
