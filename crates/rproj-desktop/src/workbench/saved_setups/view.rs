use super::*;
use controller::Intent;
use gpui_kit::{
    component::{button::*, checkbox::Checkbox, input::Input},
    prelude::FluentBuilder,
};
use rproj_core::catalog::{artifacts, capabilities, wally_packages};

impl Workbench {
    pub(in super::super) fn saved_entry(&self, cx: &mut Context<Self>) -> AnyElement {
        Button::new("saved-setups")
            .ghost()
            .w_full()
            .icon(IconName::FolderOpen)
            .label("Saved Setups…")
            .selected(self.saved.visible)
            .on_click(cx.listener(|this, _, w, cx| this.show_saved(w, cx)))
            .into_any_element()
    }
    pub(in super::super) fn render_saved(
        &self,
        narrow: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let unavailable = match &self.saved.support {
            None => Some("Connecting to the matching Saved Setups worker…".into()),
            Some(Err(e)) => Some(e.clone()),
            Some(Ok(())) => None,
        };
        let busy = self.running.is_some() || self.teardown_uncertain();
        let mut body = div()
            .id("saved-flow")
            .role(Role::Group)
            .aria_label("Saved Setups browser and review-first editor")
            .test_support()
            .track_focus(&self.saved_focus)
            .v_flex()
            .size_full()
            .min_w_0()
            .min_h_0()
            .gap_3()
            .p_4()
            .child(
                div()
                    .h_flex()
                    .flex_wrap()
                    .gap_2()
                    .child(
                        Button::new("saved-refresh")
                            .label("Refresh storage")
                            .disabled(busy || unavailable.is_some())
                            .on_click(
                                cx.listener(|this, _, _, cx| this.start_saved(Task::Browse, cx)),
                            ),
                    )
                    .when(self.saved.draft.is_some(), |v| {
                        v.child(
                            Button::new("saved-resume")
                                .label(format!("Resume edits: {}", self.saved.name()))
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.saved.editing = true;
                                    this.saved.step = Step::Review;
                                    this.saved.naming = None;
                                    this.saved_list.reset(this.saved.row_count());
                                    cx.notify();
                                })),
                        )
                    })
                    .when(self.saved.editing, |v| {
                        v.child(Button::new("saved-browse").label("Browse setups").on_click(
                            cx.listener(|this, _, _, cx| {
                                this.saved.editing = false;
                                cx.notify();
                            }),
                        ))
                    }),
            )
            .when_some(unavailable, |v, e| v.child(div().text_sm().child(e)))
            .when_some(self.saved.error.clone(), |v, e| {
                v.child(
                    div()
                        .id("saved-error")
                        .text_sm()
                        .text_color(cx.theme().danger)
                        .max_h(rems(5.))
                        .overflow_y_scroll()
                        .child(e),
                )
            })
            .when(self.saved.pending.is_some(), |v| {
                v.child(
                    div().id("saved-loading").child(
                        "Working on the captured saved setup… Navigation remains available.",
                    ),
                )
            })
            .when_some(self.saved.result.as_ref(), |v, r| {
                v.child(
                    div()
                        .id("saved-result")
                        .text_sm()
                        .max_h(rems(5.))
                        .overflow_y_scroll()
                        .child(format!(
                            "{} · {:?}. {}",
                            r.source.display(),
                            r.status,
                            r.message.as_deref().unwrap_or(
                                "Confirmed completion. Existing projects were not modified."
                            )
                        )),
                )
            });
        if self.saved.editing {
            body = body.child(self.saved_editor(cx));
        } else {
            let query = self.saved_query.read(cx).value().to_lowercase();
            let mut list = div()
                .id("saved-browser-list")
                .v_flex()
                .gap_2()
                .min_w_0()
                .child(
                    Input::new(&self.saved_query)
                        .id("saved-search")
                        .aria_label("Search saved setup names and summaries"),
                );
            if let Some(listing) = &self.saved.listing {
                list = list.child(
                    div()
                        .id("saved-storage-path")
                        .text_xs()
                        .max_h(rems(4.))
                        .overflow_y_scroll()
                        .child(listing.root.display().to_string()),
                );
                let entries: Vec<_> = listing
                    .entries
                    .iter()
                    .filter(|e| {
                        format!("{} {}", e.name, e.summary)
                            .to_lowercase()
                            .contains(&query)
                    })
                    .collect();
                if entries.is_empty() {
                    list=list.child(if listing.entries.is_empty(){"No saved setups. New Project can save a named composition; browsing creates nothing."}else{"No saved setups match this search."});
                }
                let choices = div()
                    .id("saved-choices")
                    .v_flex()
                    .min_h_0()
                    .flex_1()
                    .overflow_y_scroll()
                    .gap_2()
                    .children(entries.into_iter().map(|e| {
                        let name = e.name.clone();
                        let captured = name.clone();
                        div()
                            .v_flex()
                            .gap_1()
                            .child(
                                Button::new(SharedString::from(format!("saved-item-{name}")))
                                    .label(name)
                                    .selected(
                                        self.saved
                                            .selected
                                            .as_ref()
                                            .is_some_and(|s| s.source.name == e.name),
                                    )
                                    .disabled(busy)
                                    .on_click(cx.listener(move |this, _, w, cx| {
                                        this.request_saved(
                                            Intent::Load(captured.clone(), LoadPurpose::Browse),
                                            w,
                                            cx,
                                        )
                                    })),
                            )
                            .child(div().text_xs().child(e.summary.clone()))
                            .when_some(e.warning.clone(), |v, w| v.child(div().text_xs().child(w)))
                    }));
                list = list.child(choices);
            } else {
                list = list.child("Load the saved setup list using Refresh storage.");
            }
            let list = if narrow {
                list.max_h(rems(12.)).flex_shrink_0()
            } else {
                list.w(rems(15.)).h_full().flex_none()
            };
            body = body.child(
                div()
                    .id(if narrow {
                        "saved-stacked"
                    } else {
                        "saved-adjacent"
                    })
                    .test_support()
                    .flex()
                    .when(narrow, |v| v.flex_col())
                    .gap_4()
                    .flex_1()
                    .min_h_0()
                    .min_w_0()
                    .child(list)
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .min_h_0()
                            .child(self.saved_details(cx)),
                    ),
            );
        }
        body.into_any_element()
    }
    fn saved_details(&self, cx: &mut Context<Self>) -> AnyElement {
        if self
            .saved
            .prepared
            .as_ref()
            .is_some_and(|p| !matches!(p.review.action, SetupAction::Save(_)))
        {
            return self.saved_review(cx);
        }
        let Some(loaded) = &self.saved.selected else {
            return div()
                .child("Select a setup to inspect its captured composition and full path.")
                .into_any_element();
        };
        let disabled = self.running.is_some() || self.teardown_uncertain();
        let stale = self.saved.reload_sources.contains(&path(&loaded.source));
        let edit = loaded.clone();
        let rename = loaded.clone();
        let duplicate = loaded.clone();
        let delete = loaded.source.clone();
        let reuse = loaded.source.clone();
        let reload = loaded.source.name.clone();
        let mut detail=div().id("saved-details").test_support().v_flex().size_full().min_h_0().overflow_y_scroll().gap_3()
            .child(div().text_lg().child(loaded.source.name.clone())).child(div().id("saved-document-path").text_sm().child(path(&loaded.source).display().to_string()))
            .child("Saved compositions affect future reuse only. Management neither changes existing projects nor installs tools; Watch stays running.")
            .when_some(loaded.problem.clone(),|v,p|v.child(p))
            .when_some(loaded.edit_problem.clone(),|v,p|v.child(format!("Editing unavailable: {p}")))
            .when_some(loaded.reuse_problem.clone(),|v,p|v.child(format!("New Project reuse unavailable: {p}")))
            .when(stale,|v|v.child("This captured source is stale or uncertain. Reload from file before retrying."))
            .child(div().h_flex().flex_wrap().gap_2()
                .child(Button::new("saved-edit").label("Edit").disabled(disabled || stale || loaded.edit_problem.is_some()).on_click(cx.listener(move|this,_,w,cx|this.request_saved(Intent::Edit(edit.clone()),w,cx))))
                .child(Button::new("saved-rename").label("Rename…").disabled(disabled || stale).on_click(cx.listener(move|this,_,w,cx|this.request_saved(Intent::Name(rename.clone(),true),w,cx))))
                .child(Button::new("saved-duplicate").label("Duplicate…").disabled(disabled || stale).on_click(cx.listener(move|this,_,w,cx|this.request_saved(Intent::Name(duplicate.clone(),false),w,cx))))
                .child(Button::new("saved-delete").label("Delete…").disabled(disabled || stale).on_click(cx.listener(move|this,_,w,cx|this.request_saved(Intent::Prepare(delete.clone(),SetupAction::Delete),w,cx))))
                .child(Button::new("saved-use").label("Use in New Project…").disabled(disabled || stale || loaded.reuse_problem.is_some()).on_click(cx.listener(move|this,_,w,cx|this.request_saved(Intent::Reuse(reuse.clone()),w,cx))))
                .child(Button::new("saved-reload").label("Reload from file").disabled(disabled).on_click(cx.listener(move|this,_,w,cx|{let replace_draft=this.saved.draft.as_ref().is_some_and(|d|d.loaded.source.name==reload);this.request_saved(Intent::Load(reload.clone(),LoadPurpose::Reload { replace_draft }),w,cx);}))))
            .when_some(loaded.graph.as_ref(),|v,g|v.child(format!("Workflow: {:?}. Provenance: {}",g.package_workflow,g.mode)).child(format!("Packages: {}",g.packages.join(", "))).child(format!("Capabilities: {:?}",g.capabilities)).child(format!("Excluded files (retained): {}",g.dropped.join(", "))));
        if let Some(rename) = self.saved.naming {
            let source = loaded.source.clone();
            let value = self.saved_name.read(cx).value().to_string();
            let valid = rproj_core::creation::validate_name(&value).is_ok();
            detail=detail.child(Input::new(&self.saved_name).id("saved-new-name").aria_label("New saved setup name"))
                .child("Names never overwrite another setup. Case-only Rename requires an intermediate distinct name.")
                .child(Button::new("saved-review-name").label(if rename{"Review Rename"}else{"Review Duplicate"}).disabled(disabled || !valid)
                    .on_click(cx.listener(move|this,_,w,cx|{let name=this.saved_name.read(cx).value().to_string();this.request_saved(Intent::Prepare(source.clone(),if rename{SetupAction::Rename(name)}else{SetupAction::Duplicate(name)}),w,cx);})));
        }
        detail.into_any_element()
    }
    fn saved_editor(&self, cx: &mut Context<Self>) -> AnyElement {
        let Some(edit) = &self.saved.draft else {
            return div()
                .child("No editing draft. Browse and choose Edit.")
                .into_any_element();
        };
        let source = edit.loaded.source.clone();
        let reload = source.name.clone();
        let blocked = self.running.is_some() || !self.saved.editable();
        let mut editor = div()
            .v_flex()
            .flex_1()
            .min_h_0()
            .min_w_0()
            .gap_2()
            .child(
                div()
                    .id("saved-draft-owner")
                    .test_support()
                    .aria_label(format!(
                        "Editing {} · {}",
                        source.name,
                        path(&source).display()
                    ))
                    .text_sm()
                    .max_h(rems(4.))
                    .overflow_y_scroll()
                    .child(format!(
                        "Editing {} · {}",
                        source.name,
                        path(&source).display()
                    )),
            )
            .child(
                div()
                    .h_flex()
                    .flex_wrap()
                    .gap_2()
                    .children(
                        [
                            (Step::Review, "Review"),
                            (Step::Dependencies, "Dependencies"),
                            (Step::Capabilities, "Capabilities"),
                            (Step::Files, "Files"),
                        ]
                        .into_iter()
                        .map(|(step, label)| {
                            Button::new(SharedString::from(format!("saved-step-{label}")))
                                .label(label)
                                .selected(self.saved.step == step)
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.saved.step = step;
                                    if step == Step::Review && this.saved.prepared.is_none() {
                                        this.prepare_saved_edit(cx);
                                    }
                                    cx.notify();
                                }))
                        }),
                    )
                    .child(
                        Button::new("saved-discard")
                            .label("Discard draft…")
                            .disabled(self.running.is_some())
                            .on_click(cx.listener(|this, _, w, cx| {
                                this.request_saved(Intent::Discard(false), w, cx)
                            })),
                    )
                    .child(
                        Button::new("saved-editor-reload")
                            .label("Reload from file")
                            .disabled(self.running.is_some())
                            .on_click(cx.listener(move |this, _, w, cx| {
                                this.request_saved(
                                    Intent::Load(
                                        reload.clone(),
                                        LoadPurpose::Reload {
                                            replace_draft: true,
                                        },
                                    ),
                                    w,
                                    cx,
                                )
                            })),
                    ),
            );
        if self.saved.step == Step::Review {
            editor = editor.child(self.saved_review(cx));
        } else {
            let content = match self.saved.step {
                Step::Dependencies => self.saved_dependencies(cx),
                Step::Capabilities => self.saved_capabilities(cx),
                Step::Files => self.saved_files(cx),
                _ => unreachable!(),
            };
            editor = editor
                .child(
                    div()
                        .id("saved-editor-scroll")
                        .test_support()
                        .flex_1()
                        .min_h_0()
                        .overflow_y_scroll()
                        .child(content),
                )
                .child(
                    Button::new("saved-prepare")
                        .label("Review changes")
                        .disabled(blocked)
                        .on_click(cx.listener(|this, _, _, cx| this.prepare_saved_edit(cx))),
                );
        }
        editor.into_any_element()
    }
    fn saved_review(&self, cx: &mut Context<Self>) -> AnyElement {
        let disabled = self.running.is_some() || self.teardown_uncertain();
        let Some(prepared) = self.saved.current_prepared() else {
            return div().v_flex().gap_3().child("Review is not prepared. Existing bytes remain unchanged until a confirmed Save.")
            .child(Button::new("saved-prepare-review").label("Prepare Review").disabled(disabled || !self.saved.editable()).on_click(cx.listener(|this,_,_,cx|this.prepare_saved_edit(cx)))).into_any_element();
        };
        let review = &prepared.review;
        let mut body=div().id("saved-review").v_flex().size_full().min_h_0().gap_2()
            .child(div().id("saved-review-owner").text_sm().max_h(rems(4.)).overflow_y_scroll().child(format!("Captured setup: {} · {}",review.source.name,path(&review.source).display())))
            .child(div().id("saved-review-summary").max_h(rems(8.)).overflow_y_scroll().v_flex().gap_2()
                .when_some(review.document.as_ref(),|v,d|v.child(if d.replacement.is_some(){"Changed saves can reformat TOML and remove comments. Unknown metadata, provenance and unrelated exclusions are preserved."}else{"No changes. Original bytes and comments will remain untouched."}).children(d.changes.iter().map(|s|div().child(s.clone()))))
                .when_some(review.destination.as_ref(),|v,p|v.child(format!("Destination: {}. Document bytes are preserved; an existing name will never be overwritten.",p.display())))
                .when(matches!(review.action,SetupAction::Delete),|v|v.child("Delete permanently removes only this saved document. Existing projects are unaffected.")))
            .child(div().h_flex().flex_wrap().gap_2()
                .child(Button::new("saved-apply").label(match review.action{SetupAction::Save(_)=>"Save changes…",SetupAction::Rename(_)=>"Rename setup…",SetupAction::Duplicate(_)=>"Duplicate setup…",SetupAction::Delete=>"Delete setup…"}).disabled(disabled || self.saved.ready().is_none()).on_click(cx.listener(|this,_,w,cx|this.confirm_saved(w,cx))))
                .when(review.document.is_some(),|v|v.child(Button::new("saved-toggle-diff").label(if self.saved.show_diff{"Hide complete diff"}else{"Show complete diff"}).on_click(cx.listener(|this,_,_,cx|{this.saved.show_diff = !this.saved.show_diff;cx.notify();}))))
                .when(!matches!(review.action,SetupAction::Save(_)),|v|v.child(Button::new("saved-return-browser").label("Keep browsing").on_click(cx.listener(|this,_,_,cx|{this.saved.prepared=None;cx.notify();})))));
        if self.saved.show_diff && review.document.is_some() {
            body = body
                .child(div().text_xs().child(
                    "Old / new line · + added · − removed · controls and line endings annotated",
                ))
                .child(super::super::diff_view::render(
                    "saved",
                    prepared.rows.clone(),
                    self.saved_list.clone(),
                    self.saved_diff_focus.clone(),
                    false,
                    cx,
                ));
        }
        body.into_any_element()
    }
    fn saved_dependencies(&self, cx: &mut Context<Self>) -> AnyElement {
        let edit = self.saved.draft.as_ref().unwrap();
        let graph = &edit.draft.graph;
        let disabled = self.running.is_some() || !self.saved.editable();
        let query = self.saved_package_query.read(cx).value().to_lowercase();
        let mut body=div().v_flex().gap_3().child("Package edits retain recorded provenance. Selections are explicit. Add suggested companions using the shared catalog rules; capability-owned testing packages are selected under Capabilities.")
            .child(div().h_flex().gap_2().children([(PackageWorkflow::Wally,"Wally"),(PackageWorkflow::None,"None")].into_iter().map(|(workflow,label)|Button::new(SharedString::from(format!("saved-workflow-{label}"))).label(label).selected(graph.package_workflow==workflow).disabled(disabled).on_click(cx.listener(move|this,_,w,cx|this.saved_workflow(workflow,w,cx))))))
            .child(Input::new(&self.saved_package_query).id("saved-package-search").aria_label("Search available packages"))
            .child(Button::new("saved-companions").label("Add suggested companions").disabled(disabled || graph.package_workflow == PackageWorkflow::None).on_click(cx.listener(|this,_,_,cx|{this.saved.add_companions();cx.notify();})));
        if graph.package_workflow == PackageWorkflow::None {
            return body
                .child(
                    "This workflow has no Wally package selections. Testing can still use TestEZ.",
                )
                .into_any_element();
        }
        for package in wally_packages::PACKAGES.iter().filter(|p| {
            !rproj_core::creation::capability_owned(p.key)
                && (query.is_empty()
                    || format!("{} {}", p.module_name, p.description)
                        .to_lowercase()
                        .contains(&query))
        }) {
            let key = package.key;
            body = body.child(
                div()
                    .v_flex()
                    .gap_1()
                    .pb_2()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(
                        Checkbox::new(SharedString::from(format!("saved-package-{key}")))
                            .label(package.module_name)
                            .checked(graph.packages.iter().any(|k| k == key))
                            .disabled(disabled)
                            .on_change(cx.listener(move |this, _, _, cx| {
                                this.saved.edit(|d| d.toggle_package(key, false));
                                cx.notify();
                            })),
                    )
                    .child(package.description)
                    .child(
                        Button::new(SharedString::from(format!("saved-detail-{key}")))
                            .ghost()
                            .label("Details and documentation")
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.saved.selected_package = Some(key.into());
                                cx.notify();
                            })),
                    ),
            );
        }
        if let Some(package) = self
            .saved
            .selected_package
            .as_deref()
            .and_then(wally_packages::find)
        {
            body = body.child(
                div()
                    .id("saved-package-detail")
                    .v_flex()
                    .gap_2()
                    .child(package.module_name)
                    .child(package.description)
                    .child(
                        gpui_kit::base::Link::new("saved-package-docs")
                            .href(package.docs_url)
                            .text_color(cx.theme().link)
                            .child("Documentation ↗"),
                    )
                    .when_some(
                        self.catalog.iter().find(|c| c.key == package.key),
                        |v, c| {
                            v.child(c.when_useful.clone())
                                .child(c.caveats.clone())
                                .child(
                                    div()
                                        .id("saved-code-example")
                                        .text_sm()
                                        .overflow_x_scroll()
                                        .child(c.example.clone()),
                                )
                        },
                    ),
            );
        }
        body.into_any_element()
    }
    fn saved_capabilities(&self, cx: &mut Context<Self>) -> AnyElement {
        let graph = &self.saved.draft.as_ref().unwrap().draft.graph;
        let disabled = self.running.is_some() || !self.saved.editable();
        let mut body = div().v_flex().gap_3();
        for cap in capabilities::CAPABILITIES {
            let key = cap.key;
            let selected = graph.capabilities.get(key);
            let missing = cap.missing_requirements(|k| graph.capabilities.contains_key(k));
            body=body.child(div().v_flex().gap_2().child(cap.outcome)
                .child(div().h_flex().flex_wrap().gap_2().child(Button::new(SharedString::from(format!("saved-cap-{key}-none"))).label("None").selected(selected.is_none()).disabled(disabled).on_click(cx.listener(move|this,_,_,cx|{this.saved.edit(|d|d.set_capability(key,None));cx.notify();})))
                    .children(cap.implementations.iter().map(|i|{let value=i.key;Button::new(SharedString::from(format!("saved-cap-{key}-{value}"))).label(i.display).selected(selected.is_some_and(|s|s==value)).disabled(disabled || !missing.is_empty() || !i.workflows.contains(&graph.package_workflow))
                        .on_click(cx.listener(move|this,_,_,cx|{this.saved.edit(|d|d.set_capability(key,Some(value)));cx.notify();}))})))
                .when(!missing.is_empty(),|v|v.child(format!("Requires: {}",missing.join(", "))))
                .when(key=="test",|v|v.child("Choose the runner/backend explicitly. Jest requires Wally; incompatible testing must be changed or disabled before Save."))
                .when_some(selected.filter(|s|cap.implementation(s).is_none_or(|i|!i.workflows.contains(&graph.package_workflow))),|v,s|v.child(format!("Incompatible selection retained: {s}. Choose another implementation or None explicitly."))));
        }
        body.into_any_element()
    }
    fn saved_files(&self, cx: &mut Context<Self>) -> AnyElement {
        let graph = &self.saved.draft.as_ref().unwrap().draft.graph;
        let env = self.saved.environment();
        let disabled = self.running.is_some() || !self.saved.editable();
        let mut body=div().v_flex().gap_3().child("Optional exclusions affect future reuse. Required files stay protected. Existing unknown or inactive exclusions remain recorded.")
            .when_some(self.saved.listing.as_ref().and_then(|l|l.environment_problem.clone()),|v,p|v.child(p));
        for file in graph.full_plan(&env.apps, &env.extensions) {
            let key = file.key;
            let required = artifacts::find(key).is_some_and(|a| !a.droppable());
            let environment = env.clone();
            body = body.child(
                div()
                    .v_flex()
                    .gap_1()
                    .child(
                        Checkbox::new(SharedString::from(format!("saved-file-{key}")))
                            .label(format!(
                                "{key}{}",
                                if required { " (required)" } else { "" }
                            ))
                            .checked(required || !graph.dropped.iter().any(|k| k == key))
                            .disabled(disabled || required)
                            .on_change(cx.listener(move |this, _, _, cx| {
                                this.saved.edit(|d| d.toggle_file(key, &environment));
                                cx.notify();
                            })),
                    )
                    .child(file.reason.describe()),
            );
        }
        body.child(format!("Retained exclusions: {}", graph.dropped.join(", ")))
            .into_any_element()
    }
}
