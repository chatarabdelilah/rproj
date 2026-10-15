use gpui_kit::component::{Theme, ThemeConfig};
use gpui_kit::{App, Rems, SharedString, rems};
use std::rc::Rc;

pub(super) const SIDEBAR_WIDTH: Rems = rems(14.);
pub(super) const PANEL_PADDING: Rems = rems(1.25);
pub(super) const GROUP_GAP: Rems = rems(1.);

// Register both palettes so OS appearance changes use Soft too. View code reads
// semantic theme roles; palette literals belong only at this boundary.
pub(super) fn install(cx: &mut App) {
    Theme::update(cx, |theme| {
        theme.light_theme = Rc::new(soft((*theme.light_theme).clone(), false));
        theme.dark_theme = Rc::new(soft((*theme.dark_theme).clone(), true));
    });
}

fn soft(mut config: ThemeConfig, dark: bool) -> ThemeConfig {
    config.name = if dark {
        "rproj Soft Dark"
    } else {
        "rproj Soft Light"
    }
    .into();
    config.radius = Some(9);
    config.radius_lg = Some(10);
    config.shadow = Some(false);
    let color = |light: &'static str, dark_value: &'static str| -> Option<SharedString> {
        Some(if dark { dark_value } else { light }.into())
    };
    let c = &mut config.colors;
    c.background = color("#fcfbf8", "#1e1e1d");
    c.foreground = color("#282925", "#efeee8");
    c.muted = color("#f1f0eb", "#272725");
    c.muted_foreground = color("#605f58", "#bbb9ae");
    c.border = color("#deded5", "#42423c");
    c.input = color("#828278", "#85857b");
    c.primary = color("#2459b8", "#97bcff");
    c.primary_foreground = color("#ffffff", "#101b2c");
    c.primary_hover = color("#184795", "#b6d0ff");
    c.primary_active = c.primary_hover.clone();
    c.ring = c.primary.clone();
    c.accent = color("#e6eefb", "#2d3c52");
    c.accent_foreground = c.foreground.clone();
    c.secondary = c.muted.clone();
    c.secondary_foreground = c.foreground.clone();
    c.secondary_hover = color("#e8e7e0", "#343430");
    c.secondary_active = c.secondary_hover.clone();
    c.selection = c.accent.clone();
    c.sidebar = c.muted.clone();
    c.sidebar_foreground = c.foreground.clone();
    c.sidebar_border = c.border.clone();
    c.sidebar_accent = c.accent.clone();
    c.sidebar_accent_foreground = c.foreground.clone();
    c.sidebar_primary = c.primary.clone();
    c.sidebar_primary_foreground = c.primary_foreground.clone();
    c.popover = c.background.clone();
    c.popover_foreground = c.foreground.clone();
    c.button = c.background.clone();
    c.button_foreground = c.foreground.clone();
    c.button_hover = c.secondary_hover.clone();
    c.button_active = c.accent.clone();
    c.button_primary = c.primary.clone();
    c.button_primary_foreground = c.primary_foreground.clone();
    c.button_primary_hover = c.primary_hover.clone();
    c.button_primary_active = c.primary_active.clone();
    c.button_secondary = c.secondary.clone();
    c.button_secondary_foreground = c.secondary_foreground.clone();
    c.button_secondary_hover = c.secondary_hover.clone();
    c.button_secondary_active = c.secondary_active.clone();
    c.list = c.background.clone();
    c.list_hover = c.secondary_hover.clone();
    c.list_active = c.accent.clone();
    c.list_active_border = c.accent.clone();
    c.danger = color("#a32c36", "#ffadb5");
    config
}

#[cfg(test)]
mod tests {
    use super::*;

    fn luminance(value: &Option<SharedString>) -> f64 {
        let rgb = u32::from_str_radix(value.as_ref().unwrap().trim_start_matches('#'), 16).unwrap();
        [16, 8, 0]
            .into_iter()
            .zip([0.2126, 0.7152, 0.0722])
            .map(|(shift, weight)| {
                let channel = ((rgb >> shift) & 255) as f64 / 255.;
                weight
                    * if channel <= 0.04045 {
                        channel / 12.92
                    } else {
                        ((channel + 0.055) / 1.055).powf(2.4)
                    }
            })
            .sum()
    }

    #[test]
    fn soft_text_and_focus_contrast_meet_accessibility_thresholds() {
        for dark in [false, true] {
            let theme = soft(ThemeConfig::default(), dark);
            let c = theme.colors;
            for surface in [&c.background, &c.sidebar, &c.accent, &c.button_hover] {
                for text in [&c.foreground, &c.muted_foreground] {
                    let a = luminance(surface);
                    let b = luminance(text);
                    assert!((a.max(b) + 0.05) / (a.min(b) + 0.05) >= 4.5);
                }
                let a = luminance(surface);
                let b = luminance(&c.ring);
                assert!((a.max(b) + 0.05) / (a.min(b) + 0.05) >= 3.);
            }
            let a = luminance(&c.primary);
            let b = luminance(&c.primary_foreground);
            assert!((a.max(b) + 0.05) / (a.min(b) + 0.05) >= 4.5);
        }
    }
}
