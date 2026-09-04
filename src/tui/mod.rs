mod layout;
mod terminal;
mod widgets;

pub use layout::{centered, is_too_small, responsive_panes};
pub use terminal::TerminalSession;
pub use widgets::{
    ConfirmState, InputState, PickerItem, PickerState, render_confirm, render_footer, render_input,
    render_picker,
};

use ratatui::style::{Color, Modifier, Style};

pub const ACCENT: Color = Color::Cyan;
pub const MUTED: Color = Color::Gray;
pub const WARNING: Color = Color::Yellow;
pub const ERROR: Color = Color::Red;

pub fn title_style() -> Style {
    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
}

pub fn selected_style(active: bool) -> Style {
    Style::default()
        .fg(Color::Black)
        .bg(if active { ACCENT } else { Color::DarkGray })
}
