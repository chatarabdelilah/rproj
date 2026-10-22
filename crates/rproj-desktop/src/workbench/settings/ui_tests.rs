use super::{SettingsOutcome, SettingsStatus, WriteState};
use crate::workbench::{Cancellation, Event, Operation, OperationEvent, WatchStatus, Workbench};
use gpui_kit::{
    AnyWindowHandle, AppContext, Entity, TestAppContext, px, size, test::TestWindowExt,
};
use serde_json::json;

#[gpui_kit::test]
fn settings_preparation_recovers_without_discarding_unless_source_is_stale(
    cx: &mut TestAppContext,
) {
    let (_project, _handle, view) = fixture(cx);
    for (message, stale) in [
        ("Settings preparation cancelled; nothing written", false),
        ("Could not start owned CLI operation", false),
        ("Revert this change; other edits can still be saved", false),
        ("The file or project changed; Reload from file", true),
    ] {
        cx.update(|cx| {
            view.update(cx, |this, cx| {
                this.settings =
                    super::tests::session(this.project.clone().unwrap(), "stylua-vscode");
                this.settings.set(0, json!(true));
                let id = this.operations.begin(
                    Operation::PrepareSettings,
                    this.settings.project.clone().unwrap(),
                );
                this.running = Some((Operation::PrepareSettings, Cancellation::default()));
                this.accept_settings_prepared(
                    id,
                    this.settings.revision,
                    Err(super::ReadFailure {
                        message: message.into(),
                        teardown: false,
                    }),
                    cx,
                );
                assert!(this.settings.dirty());
                assert_eq!(this.settings.reload_required, stale);
                assert_eq!(this.settings.editable(1), !stale);
                assert!(this.settings.ready().is_none());
                if !stale {
                    this.settings.set(1, json!(true));
                    assert_eq!(
                        this.settings.loaded.as_ref().unwrap().draft.changes[0],
                        Some(json!(true))
                    );
                    assert!(this.settings.error.is_none());
                    super::tests::review(&mut this.settings);
                    assert!(this.settings.ready().is_some());
                }
            })
        });
    }
}

fn fixture(cx: &mut TestAppContext) -> (tempfile::TempDir, AnyWindowHandle, Entity<Workbench>) {
    let (project, handle, view) = crate::workbench::ui_tests::fixture(cx);
    cx.update(|cx| {
        view.update(cx, |this, cx| {
            this.settings = super::tests::session(this.project.clone().unwrap(), "stylua-vscode");
            cx.notify();
        })
    });
    (project, handle, view)
}
fn prepare(view: &Entity<Workbench>, cx: &mut TestAppContext) {
    cx.update(|cx| {
        view.update(cx, |this, cx| {
            this.settings.set(0, json!(true));
            super::tests::review(&mut this.settings);
            this.settings_list.reset(this.settings.row_count());
            cx.notify();
        })
    });
}
fn running(view: &Entity<Workbench>, cx: &mut TestAppContext) -> u64 {
    cx.update(|cx| {
        view.update(cx, |this, cx| {
            let project = this.settings.project.clone().unwrap();
            let id = this.operations.begin(Operation::SaveSettings, project);
            this.running = Some((Operation::SaveSettings, Cancellation::default()));
            this.settings.loading = true;
            cx.notify();
            id
        })
    })
}
fn success(view: &Entity<Workbench>, cx: &mut TestAppContext) -> SettingsOutcome {
    cx.update(|cx| {
        let this = view.read(cx);
        let mut saved = this.settings.loaded.clone().unwrap();
        saved.context.original = Some("{\"editor.formatOnSave\":true}\n".into());
        saved
            .context
            .identities
            .insert(".vscode".into(), Some("parent".into()));
        saved
            .context
            .identities
            .insert(".vscode/settings.json".into(), Some("target".into()));
        saved.draft =
            rproj_core::tool_configuration::SettingsDraft::new(vec![Some(json!(true)), None]);
        SettingsOutcome {
            project: saved.context.project.clone(),
            tool: saved.context.tool.clone(),
            status: SettingsStatus::Succeeded,
            writes: WriteState::Completed,
            saved: Some(saved),
            message: None,
        }
    })
}

#[gpui_kit::test]
fn settings_loading_captures_canonical_owner_and_rejects_stale_revision(cx: &mut TestAppContext) {
    let (_project, _handle, view) = fixture(cx);
    cx.update(|cx| {
        view.update(cx, |this, cx| {
            let canonical = this.settings.project.clone().unwrap();
            let selected = canonical.join(".");
            this.settings.project = Some(selected.clone());
            this.settings.loaded = None;
            let id = this.operations.begin(Operation::LoadSettings, selected);
            this.running = Some((Operation::LoadSettings, Cancellation::default()));
            let loaded = super::tests::loaded(canonical.clone(), "stylua-vscode");
            this.accept_settings_loaded(id + 1, this.settings.revision, Ok(loaded.clone()), cx);
            assert!(this.settings.loaded.is_none());
            this.accept_settings_loaded(id, this.settings.revision, Ok(loaded), cx);
            assert_eq!(this.settings.project, Some(canonical));
            assert!(this.settings.loaded.is_some());
            assert!(this.settings.error.is_none());
        })
    });
}

#[gpui_kit::test]
fn settings_pre_spawn_failure_is_recoverable_but_teardown_failure_blocks_closing(
    cx: &mut TestAppContext,
) {
    let (_project, handle, view) = fixture(cx);
    prepare(&view, cx);
    let id = running(&view, cx);
    let mut outcome = success(&view, cx);
    outcome.status = SettingsStatus::Failed;
    outcome.writes = WriteState::NotStarted;
    outcome.saved = None;
    outcome.message = Some("Save could not start; no settings writes were made".into());
    cx.update(|cx| {
        view.update(cx, |this, cx| {
            this.accept_settings_saved(id, Ok(outcome), cx);
            assert!(this.settings.dirty());
            assert!(!this.settings.reload_required);
            assert!(!this.teardown_uncertain());
            super::tests::review(&mut this.settings);
            assert!(this.settings.ready().is_some());
        })
    });
    let id = running(&view, cx);
    cx.update_window(handle, |_, window, cx| {
        view.update(cx, |this, cx| {
            this.settings.close_pending = true;
            this.accept_settings_saved(
                id,
                Err("Owned worker teardown could not be confirmed".into()),
                cx,
            );
            assert!(this.teardown_uncertain());
            assert!(this.settings.ready().is_none());
            assert!(!this.request_close(window, cx));
            assert!(this.settings.dirty());
        })
    })
    .unwrap();
}

#[gpui_kit::test]
fn settings_all_targets_show_catalog_fields_and_choices_support_keyboard(cx: &mut TestAppContext) {
    let (_project, handle, view) = fixture(cx);
    for tool in super::tool_settings::CONFIGURABLE_TOOLS {
        cx.update(|cx| {
            view.update(cx, |this, cx| {
                this.settings = super::tests::session(this.project.clone().unwrap(), tool.key);
                cx.notify();
            })
        });
        cx.update_window(handle, |_, window, cx| {
            window.render_frame(cx);
            assert!(
                window
                    .find("settings-captured-path")
                    .label()
                    .unwrap()
                    .contains(tool.target.relative())
            );
            assert!(window.find("settings-field-0").visible());
            if let super::SettingKind::Choice { options, .. } = &tool.settings[0].kind {
                window.click("settings-value-0-0", cx);
                for _ in 0..50 {
                    if window.find("settings-value-0-1").focused() == Some(true) {
                        break;
                    }
                    window.press("tab", cx);
                }
                assert_eq!(window.find("settings-value-0-1").focused(), Some(true));
                window.press("space", cx);
                view.update(cx, |this, _| {
                    assert_eq!(
                        this.settings.loaded.as_ref().unwrap().draft.changes[0],
                        Some(json!(options[1].value))
                    )
                });
            }
        })
        .unwrap();
    }
}

#[gpui_kit::test]
fn settings_controls_revert_and_responsive_layouts_preserve_draft(cx: &mut TestAppContext) {
    let (_project, handle, view) = fixture(cx);
    for dimensions in [
        size(px(1120.), px(780.)),
        size(px(800.), px(580.)),
        size(px(1120.), px(780.)),
    ] {
        cx.simulate_window_resize(handle, dimensions);
        cx.update_window(handle, |_, window, cx| {
            window.render_frame(cx);
            assert!(
                window
                    .find(if dimensions.width == px(800.) {
                        "settings-stacked"
                    } else {
                        "settings-adjacent"
                    })
                    .visible()
            );
            assert!(window.find("settings-save").visible());
            assert!(window.find("settings-fields").bounds().size.height > px(50.));
            window.scroll(
                "settings-fields",
                gpui_kit::ScrollDelta::Pixels(gpui_kit::point(px(0.), px(1000.))),
                cx,
            );
            if !window.find("settings-value-0-true").visible() {
                let viewport = window.find("settings-fields").bounds();
                let control = window.find("settings-value-0-true").bounds();
                window.scroll(
                    "settings-fields",
                    gpui_kit::ScrollDelta::Pixels(gpui_kit::point(
                        px(0.),
                        viewport.center().y - control.center().y,
                    )),
                    cx,
                );
            }
            window.click("settings-value-0-true", cx);
            window.render_frame(cx);
        })
        .unwrap();
        cx.update(|cx| {
            assert_eq!(
                view.read(cx)
                    .settings
                    .loaded
                    .as_ref()
                    .unwrap()
                    .draft
                    .changes[0],
                Some(json!(true))
            )
        });
    }
    cx.update_window(handle, |_, window, cx| {
        window.click("settings-revert-0", cx);
        window.render_frame(cx);
    })
    .unwrap();
    cx.update(|cx| assert!(!view.read(cx).settings.dirty()));
}

#[gpui_kit::test]
fn settings_keep_dialogs_restore_focus_and_retain_original_owner(cx: &mut TestAppContext) {
    let (_project, handle, view) = fixture(cx);
    prepare(&view, cx);
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        window.click("settings-save", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        assert!(window.find("settings-keep-reviewing").visible());
        window.press("enter", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        assert_eq!(window.find("settings-flow").focused(), Some(true));
        window.click("settings-reload", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        assert!(window.find("settings-keep-draft").visible());
        window.press("escape", cx);
    })
    .unwrap();
    let original = cx.update(|cx| view.read(cx).settings.project.clone());
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        window.click("Overview", cx);
    })
    .unwrap();
    cx.update(|cx| {
        view.update(cx, |this, cx| {
            this.close_project(cx);
            assert_eq!(this.settings.project, original);
        })
    });
    cx.update_window(handle, |_, window, cx| {
        view.update(cx, |this, cx| this.show_settings(window, cx));
        window.render_frame(cx);
        assert!(
            window
                .find("settings-captured-path")
                .label()
                .unwrap()
                .contains(original.as_ref().unwrap().to_str().unwrap())
        );
    })
    .unwrap();
}

#[gpui_kit::test]
fn settings_custom_values_require_default_keep_consent(cx: &mut TestAppContext) {
    let (_project, handle, view) = fixture(cx);
    cx.update(|cx| {
        view.update(cx, |this, cx| {
            this.settings.loaded.as_mut().unwrap().draft.current[0] = Some(json!(null));
            cx.notify();
        })
    });
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        window.click("settings-value-0-true", cx);
    })
    .unwrap();
    cx.update(|cx| assert!(!view.read(cx).settings.dirty()));
    cx.update_window(handle, |_, window, cx| {
        window.click("settings-custom-0", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        assert!(window.find("settings-keep-existing").visible());
        window.press("enter", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update(|cx| assert!(!view.read(cx).settings.consent.contains(&0)));
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        window.click("settings-custom-0", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        window.click("settings-replace-existing", cx);
        window.render_frame(cx);
        window.click("settings-value-0-true", cx);
    })
    .unwrap();
    cx.update(|cx| assert!(view.read(cx).settings.dirty()));
}

#[gpui_kit::test]
fn settings_watch_and_stale_confirmations_do_not_save(cx: &mut TestAppContext) {
    let (_project, handle, view) = fixture(cx);
    prepare(&view, cx);
    cx.update(|cx| {
        view.update(cx, |this, cx| {
            this.watch = Some(WatchStatus {
                supported: true,
                project: this.settings.project.clone(),
                state: rproj_core::operations::WatchState::Watching,
                message: String::new(),
            });
            cx.notify();
        })
    });
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        window.click("settings-save", cx);
        assert!(window.try_find("settings-keep-reviewing").is_none());
    })
    .unwrap();
    cx.update(|cx| {
        view.update(cx, |this, cx| {
            this.watch = None;
            cx.notify();
        })
    });
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        window.click("settings-save", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update(|cx| view.update(cx, |this, _| this.watch_generation += 1));
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        window.click("settings-confirm-save", cx);
    })
    .unwrap();
    cx.update(|cx| {
        assert!(view.read(cx).running.is_none());
        assert!(
            view.read(cx)
                .settings
                .error
                .as_ref()
                .unwrap()
                .contains("expired")
        );
    });
}

#[gpui_kit::test]
fn settings_save_retains_activity_and_baseline_after_navigation(cx: &mut TestAppContext) {
    let (_project, handle, view) = fixture(cx);
    prepare(&view, cx);
    let id = running(&view, cx);
    let outcome = success(&view, cx);
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        assert!(window.try_find("cancel-operation").is_none());
        window.click("Dependencies", cx);
    })
    .unwrap();
    cx.update(|cx| {
        view.update(cx, |this, cx| {
            assert!(!this.changing_watch());
            this.operations.expanded = false;
            this.activity_height = px(280.);
            this.sender
                .send(Event::Progress(
                    id,
                    OperationEvent::Output {
                        stderr: false,
                        text: "saved fixture output\n".into(),
                    },
                ))
                .unwrap();
            this.sender
                .send(Event::SettingsSaved(id, Ok(outcome)))
                .unwrap();
            this.drain(cx);
            assert!(!this.settings.dirty());
            assert!(!this.operations.expanded);
            assert_eq!(this.activity_height, px(280.));
            assert!(
                this.activity
                    .displayed()
                    .any(|l| l.contains("saved fixture output")
                        && l.contains(this.settings.project.as_ref().unwrap().to_str().unwrap()))
            );
            assert_eq!(
                this.settings.loaded.as_ref().unwrap().draft.current[0],
                Some(json!(true))
            );
        })
    });
}

#[gpui_kit::test]
fn settings_uncertain_save_and_stale_events_keep_edits(cx: &mut TestAppContext) {
    let (_project, _handle, view) = fixture(cx);
    prepare(&view, cx);
    let id = running(&view, cx);
    let mut outcome = success(&view, cx);
    outcome.status = SettingsStatus::Uncertain;
    outcome.saved = None;
    outcome.writes = WriteState::Unknown;
    cx.update(|cx| {
        view.update(cx, |this, cx| {
            this.accept_settings_saved(id + 1, Ok(outcome.clone()), cx);
            assert!(this.running.is_some());
            this.settings.close_pending = true;
            this.accept_settings_saved(id, Ok(outcome), cx);
            assert!(this.settings.dirty());
            assert!(this.settings.reload_required);
            assert!(!this.settings.close_pending);
            assert!(this.settings.ready().is_none());
            assert!(this.operations.expanded);
        })
    });
}

#[gpui_kit::test]
fn settings_close_waits_for_save_and_then_protects_other_dirty_drafts(cx: &mut TestAppContext) {
    let (_project, handle, view) = fixture(cx);
    prepare(&view, cx);
    let id = running(&view, cx);
    let outcome = success(&view, cx);
    cx.update(|cx| view.update(cx, |this, _| this.creation.dirty = true));
    cx.update_window(handle, |_, window, cx| {
        view.update(cx, |this, cx| assert!(!this.request_close(window, cx)));
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        assert!(window.find("settings-keep-open").visible());
        window.click("settings-close-after-save", cx);
    })
    .unwrap();
    cx.update(|cx| {
        view.update(cx, |this, cx| {
            assert!(this.settings.close_pending);
            assert!(!this.running.as_ref().unwrap().1.is_cancelled());
            this.accept_settings_saved(id, Ok(outcome), cx);
        })
    });
    cx.run_until_parked();
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        assert!(window.find("keep-draft").visible());
        window.press("escape", cx);
    })
    .unwrap();
}

#[gpui_kit::test]
fn settings_integer_validation_keeps_exact_limits(cx: &mut TestAppContext) {
    let (_project, handle, view) = fixture(cx);
    cx.update(|cx| {
        view.update(cx, |this, cx| {
            this.settings = super::tests::session(this.project.clone().unwrap(), "stylua");
            cx.notify();
        })
    });
    cx.update_window(handle, |_, window, cx| {
        // The native input dialog is opened by the same controller as its field button.
        view.update(cx, |this, cx| this.edit_settings_integer(1, window, cx));
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        window.click("settings-integer-input", cx);
        window.press("ctrl-a", cx);
        window.input("9223372036854775808", cx);
        window.click("settings-choose-integer", cx);
        assert!(window.find("settings-keep-integer").visible());
        window.click("settings-integer-input", cx);
        window.press("ctrl-a", cx);
        window.input("-9223372036854775808", cx);
        window.click("settings-choose-integer", cx);
    })
    .unwrap();
    cx.update(|cx| {
        assert_eq!(
            view.read(cx)
                .settings
                .loaded
                .as_ref()
                .unwrap()
                .draft
                .changes[1],
            Some(json!(i64::MIN))
        )
    });
}

#[gpui_kit::test]
fn settings_loading_errors_blocked_fields_and_unsupported_worker_keep_interface(
    cx: &mut TestAppContext,
) {
    let (_project, handle, view) = fixture(cx);
    cx.update(|cx| {
        view.update(cx, |this, cx| {
            this.settings.loading = true;
            cx.notify();
        })
    });
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        assert!(window.find("settings-loading").visible());
        assert!(window.find("Tools").visible());
    })
    .unwrap();
    cx.update(|cx| {
        view.update(cx, |this, cx| {
            this.settings.loading = false;
            this.settings.error = Some("Malformed JSON; repair it and reload.".into());
            this.settings
                .loaded
                .as_mut()
                .unwrap()
                .blocked
                .insert(0, "The replacement key already exists.".into());
            cx.notify();
        })
    });
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        assert!(window.find("settings-error").visible());
        window.click("settings-value-0-true", cx);
    })
    .unwrap();
    cx.update(|cx| {
        view.update(cx, |this, cx| {
            assert!(!this.settings.dirty());
            this.settings.support = Some(Err("Use a matching distribution".into()));
            this.settings.visible = false;
            this.section = crate::workbench::Section::Tools;
            cx.notify();
        })
    });
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        assert!(window.find("configure-settings").visible());
        window.click("configure-settings", cx);
        assert!(window.try_find("settings-flow").is_none());
    })
    .unwrap();
}

#[gpui_kit::test]
fn settings_completion_during_close_dialog_preserves_other_drafts(cx: &mut TestAppContext) {
    let (_project, handle, view) = fixture(cx);
    prepare(&view, cx);
    let id = running(&view, cx);
    let outcome = success(&view, cx);
    cx.update(|cx| view.update(cx, |this, _| this.creation.dirty = true));
    cx.update_window(handle, |_, window, cx| {
        view.update(cx, |this, cx| assert!(!this.request_close(window, cx)));
    })
    .unwrap();
    cx.run_until_parked();
    cx.update(|cx| {
        view.update(cx, |this, cx| {
            this.accept_settings_saved(id, Ok(outcome), cx)
        })
    });
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        window.click("settings-close-after-save", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        assert!(window.find("keep-draft").visible());
        window.press("escape", cx);
    })
    .unwrap();
}

#[gpui_kit::test]
fn settings_wrapped_diff_keeps_all_rows_accessible_in_both_themes(cx: &mut TestAppContext) {
    let (_project, handle, view) = fixture(cx);
    prepare(&view, cx);
    let count = cx.update(|cx| {
        view.update(cx, |this, cx| {
            let prepared = this.settings.prepared.as_mut().unwrap();
            prepared.review.contents = Some(
                (0..900)
                    .map(|i| format!("long_configuration_{i} = \"{}\"\r\n", "value ".repeat(60)))
                    .collect(),
            );
            prepared.rows = std::sync::Arc::new(prepared.review.diff().unwrap());
            let count = prepared.rows.len();
            this.settings_list.reset(count);
            cx.notify();
            count
        })
    });
    for mode in [
        gpui_kit::component::ThemeMode::Light,
        gpui_kit::component::ThemeMode::Dark,
    ] {
        cx.update_window(handle, |_, window, cx| {
            gpui_kit::component::Theme::change(mode, Some(window), cx);
            window.render_frame(cx);
            assert!(window.find("settings-diff").bounds().size.height > px(50.));
            assert!(
                window
                    .try_find(gpui_kit::SharedString::from(format!(
                        "settings-row-{}",
                        count - 1
                    )))
                    .is_none()
            );
            window.click("settings-diff", cx);
            window.press("end", cx);
            window.render_frame(cx);
            assert!(
                window
                    .find(gpui_kit::SharedString::from(format!(
                        "settings-row-{}",
                        count - 1
                    )))
                    .visible()
            );
            window.press("home", cx);
            window.render_frame(cx);
        })
        .unwrap();
    }
}
