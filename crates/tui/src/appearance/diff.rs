//! Diff surfaces retain add/remove semantics while adapting to a light terminal.
use ratatui::{buffer::Cell, style::Color};

pub(super) const ADDED: Color = Color::Rgb(230, 244, 232);
pub(super) const REMOVED: Color = Color::Rgb(251, 231, 233);
pub(super) const REMOVED_MARK: Color = Color::Rgb(174, 42, 62);

pub(super) fn apply_light(cell: &mut Cell) {
    let edit = match cell.bg {
        crate::diff::ADDED => Some(ADDED),
        crate::diff::REMOVED => Some(REMOVED),
        _ => None,
    };
    if let Some(background) = edit {
        cell.bg = background;
        cell.fg = match cell.fg {
            Color::LightGreen => Color::Rgb(22, 106, 53),
            Color::LightRed => REMOVED_MARK,
            color => color,
        };
    }
    cell.fg = match cell.fg {
        Color::Rgb(125, 140, 155) => Color::Rgb(95, 105, 117),
        Color::Rgb(180, 142, 220) => Color::Rgb(123, 77, 164),
        color => color,
    };
}
