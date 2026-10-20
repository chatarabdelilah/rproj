use super::*;
use gpui_kit::prelude::FluentBuilder;
use rproj_core::upgrade::diff::{Kind, Row};

pub(super) fn render(
    prefix: &'static str,
    rows: Arc<Vec<Row>>,
    state: ListState,
    focus: FocusHandle,
    unwrapped: bool,
    cx: &mut App,
) -> AnyElement {
    let ring = cx.theme().ring;
    let clicked_focus = focus.clone();
    let key_focus = focus.clone();
    let keys = state.clone();
    let count = rows.len();
    div()
        .id(SharedString::from(format!("{prefix}-diff")))
        .role(Role::Group)
        .aria_label("Read-only unified diff; arrow keys and Page Up/Down scroll")
        .test_support()
        .track_focus(&focus)
        .border_1()
        .border_color(cx.theme().border)
        .focus_visible(move |style| style.border_color(ring))
        .w_full()
        .flex_1()
        .min_h_0()
        .overflow_hidden()
        .on_mouse_down(MouseButton::Left, move |_, window, cx| {
            clicked_focus.focus(window, cx)
        })
        .on_key_down(move |event: &KeyDownEvent, window, cx| {
            if !key_focus.is_focused(window) || count == 0 {
                return;
            }
            let index = keys.logical_scroll_top().item_ix;
            let next = match event.keystroke.key.as_str() {
                "down" => index.saturating_add(1),
                "up" => index.saturating_sub(1),
                "pagedown" => index.saturating_add(10),
                "pageup" => index.saturating_sub(10),
                "home" => 0,
                "end" => count - 1,
                _ => return,
            };
            // Logical offsets reach rows that the viewport has not measured yet.
            keys.scroll_to(ListOffset {
                item_ix: next.min(count - 1),
                offset_in_item: px(0.),
            });
            cx.stop_propagation();
            window.refresh();
        })
        .child(
            list(state, move |index, _, cx| {
                let Some(row) = rows.get(index) else {
                    return div().into_any_element();
                };
                let old = row
                    .old_line
                    .map(|n| n.to_string())
                    .unwrap_or_else(|| "·".into());
                let new = row
                    .new_line
                    .map(|n| n.to_string())
                    .unwrap_or_else(|| "·".into());
                let color = match row.kind {
                    Kind::Removed => cx.theme().danger,
                    Kind::Added => cx.theme().primary,
                    Kind::Meta => cx.theme().muted_foreground,
                    Kind::Context => cx.theme().foreground,
                };
                div()
                    .id(SharedString::from(format!("{prefix}-row-{index}")))
                    .role(Role::Group)
                    .aria_label(format!("{old} / {new} {}", row.text))
                    .test_support()
                    .h_flex()
                    .items_start()
                    .gap_2()
                    .px_2()
                    .py_1()
                    .text_sm()
                    .text_color(color)
                    .child(div().w(px(72.)).flex_none().child(format!("{old} / {new}")))
                    .child(
                        div()
                            .id(SharedString::from(format!("{prefix}-row-text-{index}")))
                            .min_w_0()
                            .flex_1()
                            .whitespace_normal()
                            .when(unwrapped, |v| v.whitespace_nowrap().overflow_x_scroll())
                            .child(row.text.clone()),
                    )
                    .into_any_element()
            })
            .size_full(),
        )
        .into_any_element()
}
