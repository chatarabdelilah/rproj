use ratatui::layout::{Constraint, Direction, Layout, Rect};

pub const MIN_WIDTH: u16 = 60;
pub const MIN_HEIGHT: u16 = 16;

pub fn is_too_small(area: Rect) -> bool {
    area.width < MIN_WIDTH || area.height < MIN_HEIGHT
}

pub fn responsive_panes(area: Rect, wide_percent: u16) -> [Rect; 2] {
    let panes = if area.width >= 100 {
        Layout::horizontal([
            Constraint::Percentage(wide_percent),
            Constraint::Percentage(100 - wide_percent),
        ])
        .split(area)
    } else {
        Layout::vertical([Constraint::Percentage(48), Constraint::Percentage(52)]).split(area)
    };
    [panes[0], panes[1]]
}

pub fn centered(area: Rect, width: u16, height: u16) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(area.height.saturating_sub(height) / 2),
            Constraint::Length(height.min(area.height)),
            Constraint::Min(0),
        ])
        .split(area);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(area.width.saturating_sub(width) / 2),
            Constraint::Length(width.min(area.width)),
            Constraint::Min(0),
        ])
        .split(vertical[1])[1]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn responsive_layout_changes_at_one_hundred_columns() {
        let wide = responsive_panes(Rect::new(0, 0, 120, 30), 45);
        let narrow = responsive_panes(Rect::new(0, 0, 80, 30), 45);
        assert_eq!(wide[0].height, 30);
        assert!(wide[0].width < wide[1].width);
        assert_eq!(narrow[0].width, 80);
        assert!(narrow[0].height < narrow[1].height);
    }
}
