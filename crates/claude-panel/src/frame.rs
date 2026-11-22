//! Encode Ratatui cells for Claude Code's Raster and Text elements.
use base64::{Engine as _, engine::general_purpose::STANDARD};
use ratatui::{
    buffer::Cell,
    style::{Color, Modifier},
};
use serde::Serialize;
use tui::panel::Snapshot;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

const DEFAULT_COLOR: u32 = 0x01000000;

#[derive(Serialize)]
pub struct Frame {
    pub width: u16,
    pub position: usize,
    pub total: usize,
    pub rows: Vec<Row>,
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Row {
    Raster { cells: String },
    Text { runs: Vec<Run> },
}

#[derive(Serialize)]
pub struct Run {
    text: String,
    #[serde(flatten)]
    style: Style,
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize)]
struct Style {
    fg: Option<Foreground>,
    bg: Option<u32>,
    bold: bool,
    italic: bool,
    dim: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(untagged)]
enum Foreground {
    Rgb(u32),
    Theme(ThemeColor),
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
enum ThemeColor {
    Suggestion,
    Subtle,
    Success,
}

impl From<Snapshot> for Frame {
    fn from(snapshot: Snapshot) -> Self {
        let width = snapshot.buffer.area.width;
        Self {
            width,
            position: snapshot.position,
            total: snapshot.total,
            rows: snapshot
                .buffer
                .content()
                .chunks_exact(width.into())
                .map(draw_row)
                .collect(),
        }
    }
}

fn draw_row(row: &[Cell]) -> Row {
    if let Some(cells) = row
        .iter()
        .map(|cell| {
            let mut chars = cell.symbol().chars();
            let char = chars.next()?;
            let remaining = cell.modifier - Modifier::REVERSED;
            let style = Style::from(cell);
            let fg = match style.fg {
                Some(Foreground::Theme(_)) => return None,
                Some(Foreground::Rgb(rgb)) => rgb,
                None => DEFAULT_COLOR,
            };
            (chars.next().is_none()
                && char.len_utf16() == 1
                && char.width() == Some(1)
                && remaining.is_empty())
            .then_some((char, fg, style.bg.unwrap_or(DEFAULT_COLOR)))
        })
        .collect::<Option<Vec<_>>>()
    {
        let bytes: Vec<_> = cells
            .into_iter()
            .flat_map(|(char, fg, bg)| [char as u32, fg, bg].into_iter().flat_map(u32::to_le_bytes))
            .collect();
        return Row::Raster {
            cells: STANDARD.encode(bytes),
        };
    }

    // Raster 只接受具体 RGB 和基本多文种平面内的单格字符，主题色及其他字符由 Text 绘制。
    let mut runs: Vec<Run> = Vec::new();
    let mut column = 0;
    while let Some(cell) = row.get(column) {
        let style = Style::from(cell);
        let symbol = cell.symbol();
        let text = if symbol.is_empty() { " " } else { symbol };
        if let Some(run) = runs.last_mut()
            && run.style == style
        {
            run.text.push_str(text);
        } else {
            runs.push(Run {
                text: text.into(),
                style,
            });
        }
        column += text.width().max(1);
    }
    Row::Text { runs }
}

impl From<&Cell> for Style {
    fn from(cell: &Cell) -> Self {
        let (fg, bg) = if cell.modifier.contains(Modifier::REVERSED) {
            (cell.bg, cell.fg)
        } else {
            (cell.fg, cell.bg)
        };
        Self {
            fg: foreground(fg),
            bg: rgb(bg),
            bold: cell.modifier.contains(Modifier::BOLD),
            italic: cell.modifier.contains(Modifier::ITALIC),
            dim: cell.modifier.contains(Modifier::DIM),
        }
    }
}

fn foreground(color: Color) -> Option<Foreground> {
    let theme = match color {
        Color::LightBlue => ThemeColor::Suggestion,
        Color::DarkGray => ThemeColor::Subtle,
        Color::LightGreen => ThemeColor::Success,
        _ => return rgb(color).map(Foreground::Rgb),
    };
    Some(Foreground::Theme(theme))
}

fn rgb(color: Color) -> Option<u32> {
    const PALETTE: [u32; 16] = [
        0x000000, 0x800000, 0x008000, 0x808000, 0x000080, 0x800080, 0x008080, 0xc0c0c0, 0x808080,
        0xff0000, 0x00ff00, 0xffff00, 0x0000ff, 0xff00ff, 0x00ffff, 0xffffff,
    ];
    let index = match color {
        Color::Reset => return None,
        Color::Black => 0,
        Color::Red => 1,
        Color::Green => 2,
        Color::Yellow => 3,
        Color::Blue => 4,
        Color::Magenta => 5,
        Color::Cyan => 6,
        Color::Gray => 7,
        Color::DarkGray => 8,
        Color::LightRed => 9,
        Color::LightGreen => 10,
        Color::LightYellow => 11,
        Color::LightBlue => 12,
        Color::LightMagenta => 13,
        Color::LightCyan => 14,
        Color::White => 15,
        Color::Rgb(r, g, b) => return Some(pack(r, g, b)),
        Color::Indexed(index) => index,
    };
    Some(match index {
        0..=15 => PALETTE[usize::from(index)],
        16..=231 => {
            let cube = index - 16;
            let level = |n: u8| if n == 0 { 0 } else { 55 + n * 40 };
            pack(level(cube / 36), level((cube / 6) % 6), level(cube % 6))
        }
        232..=255 => {
            let gray = 8 + (index - 232) * 10;
            pack(gray, gray, gray)
        }
    })
}

fn pack(r: u8, g: u8, b: u8) -> u32 {
    (u32::from(r) << 16) | (u32::from(g) << 8) | u32::from(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn themed_foreground_inherits_from_claude_while_selection_keeps_its_background() {
        let mut cell = Cell::default();
        cell.set_symbol("─")
            .set_fg(Color::LightBlue)
            .set_bg(Color::Rgb(25, 65, 95));
        let row = serde_json::to_value(draw_row(&[cell])).unwrap();
        assert_eq!(row["kind"], "text");
        assert_eq!(row["runs"][0]["fg"], "suggestion");
        assert_eq!(row["runs"][0]["bg"], 0x19415f);
    }

    #[test]
    fn supplementary_file_icons_use_text_without_losing_cell_content() {
        let mut icon = Cell::default();
        icon.set_symbol("󰈙");
        let row = draw_row(&[icon, Cell::default()]);
        let Row::Text { runs } = row else {
            panic!("补充平面图标必须使用 Text");
        };
        assert_eq!(runs[0].text, "󰈙 ");
    }
}
