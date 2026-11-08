use super::*;
use gpui_kit::base::text::{TextView, TextViewStyle};
use gpui_kit::component::{button::*, input::Input};
use gpui_kit::prelude::FluentBuilder;

fn token(prefix: &str, id: &EntryId) -> SharedString {
    format!("{prefix}-{:?}-{}", id.kind, id.key).into()
}

impl Workbench {
    pub(in super::super) fn catalog_entry(&self, cx: &mut Context<Self>) -> AnyElement {
        Button::new("catalog")
            .ghost()
            .w_full()
            .label("Catalog…")
            .icon(gpui_kit::assets::IconName::BookOpen)
            .selected(self.library.visible && !self.browser.visible)
            .on_click(cx.listener(|this, _, window, cx| this.show_catalog(None, window, cx)))
            .into_any_element()
    }

    pub(in super::super) fn render_catalog(
        &self,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let narrow = window.viewport_size().width
            - theme::SIDEBAR_WIDTH.to_pixels(window.rem_size())
            < px(600.);
        let library = &self.library;
        let scope = div()
            .h_flex()
            .flex_wrap()
            .gap_2()
            .child(
                Button::new("catalog-scope-all")
                    .label("All sections")
                    .selected(library.scope.is_none())
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.library.scope = None;
                        this.library.filter();
                        cx.notify();
                    })),
            )
            .children(Kind::ALL.into_iter().map(|kind| {
                Button::new(SharedString::from(format!("catalog-scope-{kind:?}")))
                    .label(kind.label())
                    .selected(library.scope == Some(kind))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.library.scope = Some(kind);
                        this.library.filter();
                        cx.notify();
                    }))
            }));
        let mut list = div()
            .id("catalog-list")
            .role(Role::Group)
            .aria_label("Catalog results; arrow keys, Home and End select guides")
            .test_support()
            .track_focus(&library.rows_focus)
            .v_flex()
            .min_w_0()
            .min_h_0()
            .gap_2()
            .p_2()
            .border_1()
            .border_color(cx.theme().border)
            .rounded_md()
            .focus_visible({
                let ring = cx.theme().ring;
                move |s| s.border_color(ring)
            })
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                if !this.library.rows_focus.contains_focused(window, cx) {
                    return;
                }
                let n = this.library.indices.len();
                if n == 0 {
                    return;
                }
                let current = (0..n).find(|&i| {
                    this.library
                        .at(i)
                        .is_some_and(|e| e.id == this.library.selected)
                });
                let index = match event.keystroke.key.as_str() {
                    "up" => current.map(|i| i.saturating_sub(1)).unwrap_or(n - 1),
                    "down" => current.map(|i| (i + 1).min(n - 1)).unwrap_or(0),
                    "home" => 0,
                    "end" => n - 1,
                    _ => return,
                };
                let id = this.library.at(index).unwrap().id.clone();
                this.library.rows_focus.focus(window, cx);
                this.select_catalog(id, this.library.rows_focus.clone(), cx);
                this.library
                    .scroll
                    .scroll_to_item(index, ScrollStrategy::Nearest);
                cx.stop_propagation();
            }))
            .child(format!("{} entries", library.indices.len()));
        if library.indices.is_empty() {
            list = list.child(
                div()
                    .id("catalog-no-matches")
                    .role(Role::Status)
                    .aria_label(
                        "No entries match these filters. The selected guide remains available.",
                    )
                    .test_support()
                    .child("No entries match these filters. The selected guide remains available."),
            );
        } else {
            list = list.child(
                uniform_list(
                    "catalog-rows",
                    library.indices.len(),
                    cx.processor(|this, range: std::ops::Range<usize>, _, cx| {
                        range
                            .filter_map(|index| {
                                this.library.at(index).map(|entry| {
                                    let id = entry.id.clone();
                                    Button::new(token("catalog-row", &id))
                                        .ghost()
                                        .w_full()
                                        .h(px(56.))
                                        .tab_stop(false)
                                        .selected(this.library.selected == id)
                                        .child(
                                            div()
                                                .v_flex()
                                                .min_w_0()
                                                .overflow_hidden()
                                                .child(div().truncate().child(entry.name.clone()))
                                                .child(
                                                    div()
                                                        .truncate()
                                                        .text_xs()
                                                        .text_color(cx.theme().muted_foreground)
                                                        .child(format!(
                                                            "{} · {}",
                                                            entry.id.kind.label(),
                                                            entry.kind_label
                                                        )),
                                                ),
                                        )
                                        .on_click(cx.listener(move |this, _, window, cx| {
                                            this.library.rows_focus.focus(window, cx);
                                            this.select_catalog(
                                                id.clone(),
                                                this.library.rows_focus.clone(),
                                                cx,
                                            );
                                        }))
                                })
                            })
                            .collect::<Vec<_>>()
                    }),
                )
                .track_scroll(&library.scroll)
                .flex_1()
                .min_h_0()
                .w_full(),
            );
        }
        let mut detail = div()
            .id("catalog-guide")
            .role(Role::Group)
            .aria_label("Selected Catalog guide")
            .test_support()
            .min_w_0()
            .min_h_0()
            .v_flex()
            .gap_3()
            .p_3()
            .overflow_y_scroll()
            .track_scroll(&library.guide_scroll);
        if let Some(guide) = &library.guide {
            detail = detail.child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(format!(
                        "{} · {}",
                        guide.entry.id.kind.label(),
                        guide.entry.kind_label
                    )),
            );
            if !library.selection_visible() {
                detail = detail.child(div().id("catalog-filtered-selection").role(Role::Status)
                    .aria_label("Selected guide is outside the current filters").test_support()
                    .child("Selected guide is outside the current filters. Change filters to show it in the list."));
            }
            detail = detail.child(
                div()
                    .h_flex()
                    .flex_wrap()
                    .gap_2()
                    .children(guide.links.iter().enumerate().map(|(index, link)| {
                        let id = guide.entry.id.clone();
                        Button::new(SharedString::from(format!("catalog-doc-{index}")))
                            .label(format!("{} ↗", link.label))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.open_catalog_docs(&id, index, cx)
                            }))
                    }))
                    .children(guide.examples.iter().enumerate().map(|(index, _)| {
                        let id = guide.entry.id.clone();
                        Button::new(SharedString::from(format!("catalog-copy-{index}")))
                            .label(if library.copied == Some(index) {
                                "Copy requested"
                            } else {
                                "Copy example"
                            })
                            .icon(gpui_kit::assets::IconName::Copy)
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.copy_catalog_example(&id, index, cx)
                            }))
                    })),
            );
            if !guide.related.is_empty() {
                detail = detail
                    .child("Related guidance · not unconditional requirements")
                    .child(div().h_flex().flex_wrap().gap_2().children(
                        guide.related.iter().filter_map(|id| {
                            let entry = library.entries.iter().find(|entry| &entry.id == id)?;
                            let focus = library.related_focus.get(id)?.clone();
                            let target = id.clone();
                            let key_target = target.clone();
                            let click_focus = focus.clone();
                            let key_focus = focus.clone();
                            let label = format!("{} · {}", entry.name, entry.kind_label);
                            Some(
                                div()
                                    .id(token("catalog-related-focus", id))
                                    .role(Role::Button)
                                    .aria_label(label.clone())
                                    .test_support()
                                    .track_focus(&focus)
                                    .rounded_md()
                                    .border_1()
                                    .border_color(gpui_kit::transparent_black())
                                    .focus_visible({
                                        let ring = cx.theme().ring;
                                        move |s| s.border_color(ring)
                                    })
                                    .on_key_down(cx.listener(
                                        move |this, event: &KeyDownEvent, window, cx| {
                                            if key_focus.is_focused(window)
                                                && matches!(
                                                    event.keystroke.key.as_str(),
                                                    "enter" | "space"
                                                )
                                            {
                                                this.select_catalog(
                                                    key_target.clone(),
                                                    key_focus.clone(),
                                                    cx,
                                                );
                                                this.library.rows_focus.focus(window, cx);
                                                cx.stop_propagation();
                                            }
                                        },
                                    ))
                                    .child(
                                        Button::new(token("catalog-related", id))
                                            .label(label)
                                            .tab_stop(false)
                                            .on_click(cx.listener(move |this, _, window, cx| {
                                                this.select_catalog(
                                                    target.clone(),
                                                    click_focus.clone(),
                                                    cx,
                                                );
                                                this.library.rows_focus.focus(window, cx);
                                            })),
                                    ),
                            )
                        }),
                    ));
            }
            let mut code_style = StyleRefinement::default();
            code_style.text.white_space = Some(WhiteSpace::Normal);
            let style = TextViewStyle::default()
                .with_foreground(cx.theme().foreground)
                .with_muted_foreground(cx.theme().muted_foreground)
                .with_link(cx.theme().link)
                .with_selection(cx.theme().selection)
                .with_code_background(cx.theme().muted)
                .with_border(cx.theme().border)
                .with_code_block(code_style);
            detail = detail.child(TextView::new(&library.text).selectable(true).style(style));
        } else {
            detail = detail.child(div().id("catalog-unavailable").role(Role::Status)
                .aria_label("Guidance is unavailable for this entry").test_support()
                .child("Guidance is unavailable for this entry. Choose another bundled catalog entry."));
        }
        let panes = div()
            .id("catalog-panes")
            .role(Role::Group)
            .aria_label(if narrow {
                "Stacked catalog panes"
            } else {
                "Adjacent catalog panes"
            })
            .test_support()
            .min_w_0()
            .gap_4()
            .when(narrow, |s| s.v_flex().flex_none())
            .when(!narrow, |s| s.h_flex().flex_1().min_h(px(260.)))
            .child(
                list.when(narrow, |s| s.h(px(190.)))
                    .when(!narrow, |s| s.w(px(220.)).flex_none().h_full()),
            )
            .child(
                detail
                    .when(narrow, |s| s.h(px(400.)))
                    .when(!narrow, |s| s.flex_1().h_full()),
            );
        div()
            .id("catalog-browser")
            .role(Role::Group)
            .aria_label("Bundled Catalog")
            .test_support()
            .track_focus(&library.focus)
            .size_full()
            .min_w_0()
            .min_h_0()
            .v_flex()
            .overflow_y_scroll()
            .track_scroll(&library.page_scroll)
            .gap_3()
            .p(theme::PANEL_PADDING)
            .child(
                div()
                    .h_flex()
                    .gap_2()
                    .child(
                        Button::new("catalog-back")
                            .label("Back")
                            .disabled(library.history.is_empty())
                            .on_click(
                                cx.listener(|this, _, window, cx| this.catalog_back(window, cx)),
                            ),
                    )
                    .child(div().flex_1().min_w_0().child(Input::new(&library.query))),
            )
            .child("Bundled guidance · browsing does not install or change anything")
            .child(scope)
            .child(panes)
            .into_any_element()
    }
}
