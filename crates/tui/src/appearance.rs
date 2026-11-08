//! Terminal-reported appearance adapts interface contrast while preserving semantic color roles.
mod diff;
pub(crate) mod probe;
use ratatui::{
    buffer::Buffer,
    style::{Color, Modifier},
};

pub(crate) const SELECTION: Color = Color::Rgb(25, 65, 95);
const LIGHT_SELECTION: Color = Color::Rgb(196, 219, 239);
const LIGHT_PARAMETER: Color = Color::Rgb(0, 112, 190);
const LIGHT_VALUE: Color = Color::Rgb(0, 130, 94);
const SELECTED_PARAMETER: Color = Color::Rgb(0, 94, 180);
const SELECTED_VALUE: Color = Color::Rgb(0, 108, 78);

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Background {
    Light,
    Dark,
    #[default]
    Unreported,
}

impl Background {
    pub fn syntax_theme(self) -> highlight::Theme {
        match self {
            Self::Light => highlight::Theme::Light,
            Self::Dark | Self::Unreported => highlight::Theme::Dark,
        }
    }
    pub fn rgb(red: u8, green: u8, blue: u8) -> Self {
        let linear = |channel: u8| {
            let value = f64::from(channel) / 255.0;
            if value <= 0.04045 {
                value / 12.92
            } else {
                ((value + 0.055) / 1.055).powf(2.4)
            }
        };
        let luminance = 0.2126 * linear(red) + 0.7152 * linear(green) + 0.0722 * linear(blue);
        if luminance > 0.179 {
            Self::Light
        } else {
            Self::Dark
        }
    }

    pub fn apply(self, buffer: &mut Buffer) {
        for cell in &mut buffer.content {
            // 差异底色单独适配，界面语义色只调整默认底色及选中行。
            if self == Self::Light {
                diff::apply_light(cell);
                cell.fg = match (cell.bg, cell.fg) {
                    (Color::Reset, Color::LightCyan) => LIGHT_PARAMETER,
                    (Color::Reset, Color::LightGreen) => LIGHT_VALUE,
                    (SELECTION, Color::LightCyan) => SELECTED_PARAMETER,
                    (SELECTION, Color::LightGreen) => SELECTED_VALUE,
                    (_, color) => color,
                };
            }
            // 渲染层统一使用选中色标记，输出前按终端底色映射。
            if cell.bg != SELECTION {
                continue;
            }
            cell.bg = match self {
                Self::Light => LIGHT_SELECTION,
                Self::Dark => SELECTION,
                Self::Unreported => {
                    cell.modifier.insert(Modifier::BOLD | Modifier::UNDERLINED);
                    Color::Reset
                }
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::layout::Rect;

    #[test]
    fn terminal_background_uses_luminance_instead_of_terminal_name() {
        for (rgb, expected) in [
            ((255, 255, 255), Background::Light),
            ((240, 235, 222), Background::Light),
            ((128, 128, 128), Background::Light),
            ((35, 33, 54), Background::Dark),
            ((0, 0, 0), Background::Dark),
        ] {
            assert_eq!(Background::rgb(rgb.0, rgb.1, rgb.2), expected);
        }
    }

    #[test]
    fn selection_adapts_without_overwriting_text_status_or_diff_backgrounds() {
        for background in [Background::Light, Background::Dark, Background::Unreported] {
            let mut buffer = Buffer::empty(Rect::new(0, 0, 3, 1));
            buffer[(0, 0)]
                .set_symbol("x")
                .set_fg(Color::Reset)
                .set_bg(SELECTION);
            buffer[(1, 0)]
                .set_symbol("!")
                .set_fg(Color::Yellow)
                .set_bg(SELECTION);
            buffer[(1, 0)].modifier.insert(Modifier::ITALIC);
            buffer[(2, 0)].set_bg(Color::Rgb(15, 45, 25));
            let diff = buffer[(2, 0)].clone();
            background.apply(&mut buffer);
            assert_eq!(buffer[(0, 0)].fg, Color::Reset);
            assert_eq!(buffer[(1, 0)].fg, Color::Yellow);
            assert!(buffer[(1, 0)].modifier.contains(Modifier::ITALIC));
            assert_eq!(buffer[(2, 0)], diff);
            let expected = match background {
                Background::Light => LIGHT_SELECTION,
                Background::Dark => SELECTION,
                Background::Unreported => Color::Reset,
            };
            assert_eq!(buffer[(0, 0)].bg, expected);
            if background == Background::Unreported {
                assert!(buffer[(0, 0)].modifier.contains(Modifier::UNDERLINED));
            }
        }
    }

    #[test]
    fn light_semantic_colors_are_readable_on_both_plain_and_selected_rows() {
        let luminance = |rgb: (u8, u8, u8)| {
            let linear = |value: u8| {
                let value = f64::from(value) / 255.0;
                if value <= 0.04045 {
                    value / 12.92
                } else {
                    ((value + 0.055) / 1.055).powf(2.4)
                }
            };
            0.2126 * linear(rgb.0) + 0.7152 * linear(rgb.1) + 0.0722 * linear(rgb.2)
        };
        for (background, foregrounds) in [
            ((255, 255, 255), [LIGHT_PARAMETER, LIGHT_VALUE]),
            ((196, 219, 239), [SELECTED_PARAMETER, SELECTED_VALUE]),
        ] {
            for foreground in foregrounds {
                let Color::Rgb(r, g, b) = foreground else {
                    unreachable!()
                };
                let contrast = (luminance(background) + 0.05) / (luminance((r, g, b)) + 0.05);
                assert!(contrast >= 4.5, "{foreground:?}: {contrast}");
            }
        }
        let mut buffer = Buffer::empty(Rect::new(0, 0, 4, 1));
        buffer[(0, 0)].set_fg(Color::LightCyan);
        buffer[(1, 0)].set_fg(Color::LightGreen).set_bg(SELECTION);
        buffer[(2, 0)]
            .set_fg(Color::LightGreen)
            .set_bg(Color::Rgb(15, 45, 25));
        buffer[(3, 0)].set_fg(Color::Rgb(120, 210, 170));
        let original = buffer.clone();
        Background::Dark.apply(&mut buffer);
        assert_eq!(buffer, original);
        Background::Light.apply(&mut buffer);
        assert_eq!(buffer[(0, 0)].fg, LIGHT_PARAMETER);
        assert_eq!(buffer[(1, 0)].fg, SELECTED_VALUE);
        assert_eq!(buffer[(2, 0)], original[(2, 0)]);
        assert_eq!(buffer[(3, 0)], original[(3, 0)]);
    }
}
