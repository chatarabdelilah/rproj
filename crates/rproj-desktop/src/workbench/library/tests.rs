use super::super::ui_tests::fixture;
use super::super::{
    Cancellation, Event, Operation, OperationEvent, Section, WatchStatus, Workbench,
};
use super::{EntryId, HISTORY_LIMIT, Kind, browser, guide_markdown, literal};
use gpui_kit::test::TestWindowExt;
use gpui_kit::{AnyWindowHandle, AppContext, Entity, TestAppContext, point, px, size};

fn open(cx: &mut TestAppContext, handle: AnyWindowHandle, view: &Entity<Workbench>) {
    cx.update_window(handle, |_, window, cx| {
        view.update(cx, |this, cx| this.show_catalog(None, window, cx));
        window.render_frame(cx);
    })
    .unwrap();
    cx.run_until_parked();
}

#[gpui_kit::test]
fn six_sections_are_bundled_and_search_does_not_remove_the_selected_guide(cx: &mut TestAppContext) {
    let (_root, handle, view) = fixture(cx);
    cx.update(|cx| {
        view.update(cx, |this, cx| {
            this.close_project(cx);
            this.client = None;
            this.client_error = Some("Worker unavailable".into());
        })
    });
    open(cx, handle, &view);
    cx.update_window(handle, |_, window, cx| {
        assert!(window.find("catalog-browser").visible());
        assert!(
            window.try_find("catalog-row-Topic-ci").is_none(),
            "off-screen results are virtualized"
        );
        view.update(cx, |this, cx| {
            assert!(
                this.request_close(window, cx),
                "Catalog alone adds no dirty-close protection"
            )
        });
        for kind in Kind::ALL {
            window.click(format!("catalog-scope-{kind:?}"), cx);
            view.update(cx, |this, _| {
                assert!(!this.library.indices.is_empty());
                assert!(
                    this.library
                        .indices
                        .iter()
                        .all(|&i| this.library.entries[i].id.kind == kind)
                );
                assert_eq!(this.library.selected, EntryId::new(Kind::Package, "scribe"));
            });
        }
        window.click("catalog-scope-all", cx);
        view.update(cx, |this, cx| {
            this.library.query.update(cx, |input, cx| {
                input.replace_all("nothing matches this query", window, cx)
            })
        });
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        assert!(window.find("catalog-no-matches").visible());
        assert!(window.find("catalog-filtered-selection").visible());
        assert!(window.try_find("catalog-unavailable").is_none());
        view.update(cx, |this, _| {
            assert!(this.library.guide.is_some());
            assert!(this.running.is_none());
            assert!(this.operations.latest.is_none());
        });
    })
    .unwrap();
}

#[gpui_kit::test]
fn same_name_entries_keep_typed_identity_and_missing_guidance_is_distinct(cx: &mut TestAppContext) {
    let (_root, handle, view) = fixture(cx);
    open(cx, handle, &view);
    cx.update_window(handle, |_, window, cx| {
        view.update(cx, |this, cx| {
            this.library
                .query
                .update(cx, |input, cx| input.replace_all("StyLua", window, cx))
        });
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        for kind in [Kind::Tool, Kind::Settings] {
            window.click(format!("catalog-row-{kind:?}-stylua"), cx);
            view.update(cx, |this, _| {
                assert_eq!(this.library.selected, EntryId::new(kind, "stylua"))
            });
        }
        view.update(cx, |this, cx| {
            this.select_catalog(
                EntryId::new(Kind::Package, "unknown"),
                this.library.rows_focus.clone(),
                cx,
            )
        });
        window.render_frame(cx);
        assert!(window.find("catalog-unavailable").visible());
        assert!(window.try_find("catalog-no-matches").is_none());
    })
    .unwrap();
}

#[gpui_kit::test]
fn related_back_restores_filters_position_and_keyboard_focus(cx: &mut TestAppContext) {
    let (_root, handle, view) = fixture(cx);
    open(cx, handle, &view);
    let before = cx
        .update_window(handle, |_, window, cx| {
            window.render_frame(cx);
            view.update(cx, |this, cx| {
                this.library.scope = Some(Kind::Package);
                this.library.query_text = "package".into();
                this.library
                    .query
                    .update(cx, |input, cx| input.replace_all("package", window, cx));
                this.library.filter();
                this.library
                    .guide_scroll
                    .set_offset(point(px(0.), px(-30.)));
                this.library
                    .scroll
                    .0
                    .borrow()
                    .base_handle
                    .set_offset(point(px(0.), px(-56.)));
                cx.notify();
            });
            window.render_frame(cx);
            let offset = view.read(cx).library.scroll.0.borrow().base_handle.offset();
            window.click("catalog-related-Tool-scribe-studio", cx);
            assert_eq!(
                view.read(cx).library.selected,
                EntryId::new(Kind::Tool, "scribe-studio")
            );
            assert_eq!(
                view.read(cx).library.history.back().unwrap().list_offset,
                offset,
                "bookmark captured the scrolled list"
            );
            window.click("catalog-back", cx);
            offset
        })
        .unwrap();
    cx.run_until_parked();
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        let library = &view.read(cx).library;
        assert_eq!(library.selected, EntryId::new(Kind::Package, "scribe"));
        assert_eq!(library.scope, Some(Kind::Package));
        assert_eq!(library.query.read(cx).value().as_ref(), "package");
        assert_eq!(library.guide_scroll.offset().y, px(-30.));
        assert_eq!(library.scroll.0.borrow().base_handle.offset(), before);
        assert!(
            library.related_focus[&EntryId::new(Kind::Tool, "scribe-studio")].is_focused(window)
        );
        window.press("enter", cx);
        assert_eq!(
            view.read(cx).library.selected,
            EntryId::new(Kind::Tool, "scribe-studio")
        );
    })
    .unwrap();
}

#[gpui_kit::test]
fn history_is_bounded_and_sidebar_navigation_preserves_selection(cx: &mut TestAppContext) {
    let (_root, handle, view) = fixture(cx);
    open(cx, handle, &view);
    cx.update_window(handle, |_, window, cx| {
        view.update(cx, |this, cx| {
            for index in 0..42 {
                this.select_catalog(
                    EntryId::new(
                        Kind::Package,
                        if index % 2 == 0 { "react" } else { "scribe" },
                    ),
                    this.library.rows_focus.clone(),
                    cx,
                );
            }
            assert_eq!(this.library.history.len(), HISTORY_LIMIT);
            this.library.rows_focus.focus(window, cx);
        });
        window.render_frame(cx);
        window.press("end", cx);
        let selected = view.read(cx).library.selected.clone();
        assert_eq!(selected, view.read(cx).library.entries.last().unwrap().id);
        window.click("Overview", cx);
        assert!(!view.read(cx).library.visible);
        window.click("catalog", cx);
        assert_eq!(view.read(cx).library.selected, selected);
        assert!(window.find("catalog-browser").visible());
    })
    .unwrap();
}

#[gpui_kit::test]
fn dependency_links_match_canonical_identity_not_aliases(cx: &mut TestAppContext) {
    let (root, handle, view) = fixture(cx);
    let source = rproj_core::catalog::wally_packages::find("scribe")
        .unwrap()
        .source;
    std::fs::write(
        root.path().join("wally.toml"),
        format!(
            "[dependencies]\nSignal = \"{source}\"\nUnfamiliar = \"someone/unfamiliar@1.0.0\"\n"
        ),
    )
    .unwrap();
    let snapshot = rproj_core::inspect(root.path()).unwrap();
    cx.update_window(handle, |_, window, cx| {
        view.update(cx, |this, cx| {
            this.snapshot = Some(snapshot);
            let packages = &this.snapshot.as_ref().unwrap().packages;
            let mut known = packages
                .iter()
                .find(|p| p.alias == "Signal")
                .unwrap()
                .clone();
            let mut unknown = packages
                .iter()
                .find(|p| p.alias == "Unfamiliar")
                .unwrap()
                .clone();
            known.alias = "CustomDataAlias".into();
            unknown.alias = "Scribe".into();
            assert_eq!(
                this.catalog_package(&known),
                Some(EntryId::new(Kind::Package, "scribe"))
            );
            assert!(this.catalog_package(&unknown).is_none());
            this.section = Section::Dependencies;
            this.selected_package = Some("dependencies:Signal".into());
            cx.notify();
        });
        window.render_frame(cx);
        window.click("view-in-catalog", cx);
        assert_eq!(
            view.read(cx).library.selected,
            EntryId::new(Kind::Package, "scribe")
        );
        window.click("Dependencies", cx);
        window.click("dependencies:Unfamiliar", cx);
        assert!(window.try_find("view-in-catalog").is_none());
        assert!(
            view.read(cx)
                .snapshot
                .as_ref()
                .unwrap()
                .packages
                .iter()
                .any(|p| p.declaration.contains("someone/unfamiliar"))
        );
    })
    .unwrap();
}

#[gpui_kit::test]
fn complete_wrapped_selectable_guides_copy_exact_examples_and_open_exact_urls(
    cx: &mut TestAppContext,
) {
    let (_root, handle, view) = fixture(cx);
    open(cx, handle, &view);
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        window.click("catalog-copy-0", cx);
        assert_eq!(
            window.find("catalog-copy-0").label(),
            Some("Copy requested")
        );
        window.click("catalog-doc-0", cx);
    })
    .unwrap();
    let scribe = browser::guide(&EntryId::new(Kind::Package, "scribe")).unwrap();
    assert_eq!(
        cx.read_from_clipboard().unwrap().text().unwrap(),
        scribe.examples[0].code
    );
    assert_eq!(
        cx.opened_url().as_deref(),
        Some(scribe.links[0].url.as_str())
    );
    cx.update(|cx| {
        view.update(cx, |this, cx| {
            let text = this.library.text.read(cx).rendered_text();
            assert!(text.as_str().contains(&scribe.examples[0].code));
            assert!(
                text.as_str()
                    .contains(&scribe.sections.last().unwrap().text)
            );
            this.copy_catalog_example(&EntryId::new(Kind::Tool, "stylua"), 0, cx);
            this.open_catalog_docs(&EntryId::new(Kind::Tool, "stylua"), 0, cx);
        })
    });
    assert_eq!(
        cx.opened_url().as_deref(),
        Some(scribe.links[0].url.as_str())
    );
    for (width, height, label) in [
        (800., 580., "Stacked catalog panes"),
        (1120., 780., "Adjacent catalog panes"),
    ] {
        cx.simulate_window_resize(handle, size(px(width), px(height)));
        cx.update_window(handle, |_, window, cx| {
            for mode in [
                gpui_kit::component::ThemeMode::Light,
                gpui_kit::component::ThemeMode::Dark,
            ] {
                gpui_kit::component::Theme::change(mode, Some(window), cx);
                window.render_frame(cx);
                assert_eq!(window.find("catalog-panes").label(), Some(label));
                let list = window.find("catalog-list").bounds();
                let guide = window.find("catalog-guide").bounds();
                if width == 800. {
                    assert!(guide.top() >= list.bottom());
                } else {
                    assert!(guide.left() >= list.right());
                }
            }
            assert_eq!(view.read(cx).library.selected, scribe.entry.id);
            assert!(
                view.read(cx)
                    .library
                    .text
                    .read(cx)
                    .rendered_text()
                    .as_str()
                    .contains(&scribe.examples[0].code)
            );
        })
        .unwrap();
    }
}

#[gpui_kit::test]
fn catalog_preserves_project_watch_activity_and_independent_drafts_during_operations(
    cx: &mut TestAppContext,
) {
    let (root, handle, view) = fixture(cx);
    let owner = root.path().join("Owner 世界 with a long project path");
    cx.update(|cx| {
        view.update(cx, |this, cx| {
            this.watch = Some(WatchStatus {
                supported: true,
                project: Some(owner.clone()),
                state: rproj_core::operations::WatchState::Watching,
                message: "Watching".into(),
            });
            this.creation.draft = Some(rproj_core::creation::CreationDraft::new("Retained".into()));
            this.creation.dirty = true;
            this.composition.revision = 9;
            let id = this.operations.begin(Operation::Check, owner.clone());
            this.running = Some((Operation::Check, Cancellation::default()));
            this.operations.expanded = false;
            this.activity_height = px(260.);
            this.activity.set_origin(Some(owner.display().to_string()));
            this.activity.append("retained matching output");
            this.sender
                .send(Event::Progress(
                    id,
                    OperationEvent::Output {
                        stderr: false,
                        text: "more output\n".into(),
                    },
                ))
                .unwrap();
            cx.notify();
        })
    });
    open(cx, handle, &view);
    cx.update_window(handle, |_, window, cx| {
        view.update(cx, |this, cx| {
            this.filter
                .update(cx, |input, cx| input.replace_all("matching", window, cx));
            this.drain(cx);
            this.enter_project(root.path().join("Other"));
            assert!(this.library.visible);
            this.close_project(cx);
            assert!(this.library.visible);
            assert_eq!(this.watch.as_ref().unwrap().project.as_ref(), Some(&owner));
            assert_eq!(this.operations.latest.as_ref().unwrap().project, owner);
            assert!(!this.operations.expanded);
            assert_eq!(this.activity_height, px(260.));
            assert_eq!(this.filter.read(cx).value().as_ref(), "matching");
            assert!(
                this.activity
                    .displayed()
                    .any(|line| line.contains("retained matching output"))
            );
            assert_eq!(this.creation.draft.as_ref().unwrap().name, "Retained");
            assert_eq!(this.composition.revision, 9);
            this.running = None;
            assert!(
                !this.request_close(window, cx),
                "Catalog must not bypass dirty-draft protection"
            );
        });
    })
    .unwrap();
}

#[test]
fn complete_guide_text_escapes_markup_and_does_not_truncate_examples() {
    assert_eq!(
        literal("[link](url) <img>\n*text*"),
        "\\[link\\]\\(url\\) \\<img\\>  \n\\*text\\*"
    );
    for entry in browser::entries() {
        let guide = browser::guide(&entry.id).unwrap();
        let text = guide_markdown(&guide);
        for section in &guide.sections {
            assert!(text.contains(&literal(&section.text)));
        }
        for example in &guide.examples {
            assert!(text.contains(&example.code));
        }
    }
}
