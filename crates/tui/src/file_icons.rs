//! Pure file-name icon lookup, independent of Git, the filesystem, and Neovim.
mod data;
use ratatui::{
    style::{Color, Style},
    text::Span,
};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Display {
    Enabled,
    Disabled,
}
/// Expansion is independent of icon visibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Folder {
    Collapsed,
    Expanded,
}
impl Folder {
    fn glyph(self) -> Glyph {
        Glyph::new(
            match self {
                Self::Collapsed => '\u{f07b}',
                Self::Expanded => '\u{f07c}',
            },
            137,
            180,
            250,
        )
    }
    fn arrow(self) -> &'static str {
        match self {
            Self::Collapsed => "▶ ",
            Self::Expanded => "▼ ",
        }
    }
}
impl From<bool> for Display {
    fn from(enabled: bool) -> Self {
        if enabled {
            Self::Enabled
        } else {
            Self::Disabled
        }
    }
}
impl Display {
    pub fn file(self, path: &Path) -> Vec<Span<'static>> {
        match self {
            Self::Enabled => lookup(path).spans(),
            Self::Disabled => Vec::new(),
        }
    }
    pub fn directory_slot(self, folder: Folder) -> Vec<Span<'static>> {
        match self {
            Self::Enabled => folder.glyph().spans(),
            Self::Disabled => vec![Span::styled(
                folder.arrow(),
                Style::default().fg(Color::LightBlue),
            )],
        }
    }
    pub fn slot(self, path: &Path) -> Vec<Span<'static>> {
        match self {
            Self::Enabled => self.file(path),
            Self::Disabled => vec![Span::raw("  ")],
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Glyph {
    symbol: char,
    color: Color,
}
impl Glyph {
    const fn new(symbol: char, red: u8, green: u8, blue: u8) -> Self {
        Self {
            symbol,
            color: Color::Rgb(red, green, blue),
        }
    }
    fn spans(self) -> Vec<Span<'static>> {
        vec![
            Span::styled(self.symbol.to_string(), Style::default().fg(self.color)),
            Span::raw(" "),
        ]
    }
}
fn lookup(path: &Path) -> Glyph {
    let name = path
        .file_name()
        .unwrap_or(path.as_os_str())
        .to_string_lossy()
        .to_ascii_lowercase();
    if let Ok(index) = data::FILENAMES.binary_search_by(|(key, _)| key.cmp(&name.as_str())) {
        return data::FILENAMES[index].1;
    }
    // 最长复合后缀优先，例如 d.ts 优先于 ts；只检查名称，不读取文件。
    for (index, _) in name.match_indices('.') {
        let extension = &name[index + 1..];
        if let Ok(index) = data::EXTENSIONS.binary_search_by(|(key, _)| key.cmp(&extension)) {
            return data::EXTENSIONS[index].1;
        }
    }
    Glyph::new('\u{f15b}', 137, 180, 250)
}

#[cfg(test)]
mod tests {
    use super::*;
    use unicode_width::UnicodeWidthChar;

    #[test]
    fn named_files_case_and_compound_extensions_resolve_without_existing_files() {
        assert_ne!(lookup(Path::new("main.rs")), lookup(Path::new("main.ts")));
        assert_eq!(
            lookup(Path::new("/不存在/中文 图片.PNG")),
            lookup(Path::new("image.png"))
        );
        assert_eq!(
            lookup(Path::new("/not-present/DOCKERFILE")),
            lookup(Path::new("Dockerfile"))
        );
        assert_ne!(
            lookup(Path::new("types.d.ts")),
            lookup(Path::new("main.ts"))
        );
        assert_ne!(
            lookup(Path::new("README.md")),
            lookup(Path::new("article.md"))
        );
        assert_eq!(
            lookup(Path::new("unknown.unsupported")),
            lookup(Path::new("no-extension"))
        );
        assert!(Display::Disabled.file(Path::new("main.rs")).is_empty());
        assert_eq!(
            Display::Disabled.slot(Path::new("main.rs"))[0].content,
            "  "
        );
    }
    #[test]
    fn generated_tables_keep_unique_sorted_keys_and_single_cell_symbols() {
        for table in [data::FILENAMES, data::EXTENSIONS] {
            assert!(table.windows(2).all(|pair| pair[0].0 < pair[1].0));
            for (_, glyph) in table {
                assert_eq!(glyph.symbol.width(), Some(1), "{}", glyph.symbol);
            }
        }
    }
}
