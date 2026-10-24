//! Multiline UTF-8 editing with soft wrapping and cursor-aware scrolling.
use crate::Editor as LineEditor;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use unicode_width::UnicodeWidthChar;

#[derive(Debug)]
pub(crate) struct Editor {
    editor: LineEditor,
    limit: usize,
    pub scroll: usize,
    width: usize,
    reveal: Reveal,
}
#[derive(Debug)]
enum Reveal {
    Cursor,
    Keep,
}
#[derive(Debug)]
pub(crate) enum Invalid {
    ControlCharacter,
    TooLarge,
}
pub(crate) struct Layout {
    pub lines: Vec<String>,
    pub cursor: (usize, usize),
    positions: Vec<(usize, usize, usize)>,
}
impl Editor {
    pub fn new(text: String, limit: usize) -> Self {
        Self {
            editor: LineEditor::new(text),
            limit,
            scroll: 0,
            width: 80,
            reveal: Reveal::Cursor,
        }
    }
    pub fn text(&self) -> &str {
        &self.editor.text
    }
    pub fn start(&mut self) {
        self.editor.cursor = 0;
        self.reveal = Reveal::Cursor;
    }
    pub fn insert(&mut self, text: &str) -> Result<(), Invalid> {
        let text = text.replace("\r\n", "\n").replace('\r', "\n");
        if text
            .chars()
            .any(|c| c.is_control() && !matches!(c, '\n' | '\t'))
        {
            return Err(Invalid::ControlCharacter);
        }
        if self.editor.text.len() + text.len() > self.limit {
            return Err(Invalid::TooLarge);
        }
        self.editor.text.insert_str(self.editor.cursor, &text);
        self.editor.cursor += text.len();
        self.reveal = Reveal::Cursor;
        Ok(())
    }
    pub fn key(&mut self, key: KeyEvent) -> Result<(), Invalid> {
        self.reveal = Reveal::Cursor;
        let layout = self.layout(self.width);
        match key.code {
            KeyCode::Enter => self.insert("\n")?,
            KeyCode::Tab => self.insert("    ")?,
            KeyCode::Char(ch)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                self.insert(&ch.to_string())?
            }
            KeyCode::Up | KeyCode::Down | KeyCode::PageUp | KeyCode::PageDown => {
                let amount: isize = match key.code {
                    KeyCode::Up => -1,
                    KeyCode::Down => 1,
                    KeyCode::PageUp => -10,
                    _ => 10,
                };
                let row = layout.cursor.1.saturating_add_signed(amount);
                if let Some((index, _, _)) = layout
                    .positions
                    .iter()
                    .filter(|(_, y, _)| *y == row)
                    .min_by_key(|(_, _, x)| x.abs_diff(layout.cursor.0))
                {
                    self.editor.cursor = *index;
                }
            }
            KeyCode::Home | KeyCode::End if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                let row = layout.cursor.1;
                let positions = layout
                    .positions
                    .iter()
                    .filter(|(_, y, _)| *y == row)
                    .collect::<Vec<_>>();
                let target = if key.code == KeyCode::Home {
                    positions.first()
                } else {
                    positions.last()
                };
                if let Some((index, _, _)) = target {
                    self.editor.cursor = *index;
                }
            }
            _ => {
                self.editor.key(key);
            }
        }
        Ok(())
    }
    pub fn view(&mut self, width: usize, height: usize) -> Layout {
        if self.width != width.max(1) {
            self.reveal = Reveal::Cursor;
        }
        self.width = width.max(1);
        let layout = self.layout(self.width);
        if matches!(self.reveal, Reveal::Cursor) {
            if layout.cursor.1 < self.scroll {
                self.scroll = layout.cursor.1;
            }
            if layout.cursor.1 >= self.scroll + height {
                self.scroll = layout.cursor.1.saturating_sub(height.saturating_sub(1));
            }
        }
        self.reveal = Reveal::Keep;
        self.scroll = self.scroll.min(layout.lines.len().saturating_sub(height));
        layout
    }
    fn layout(&self, width: usize) -> Layout {
        let width = width.max(1);
        let mut lines = vec![String::new()];
        let mut positions = Vec::new();
        let mut column = 0usize;
        let mut cursor = (0, 0);
        for (index, ch) in self
            .editor
            .text
            .char_indices()
            .chain(std::iter::once((self.editor.text.len(), '\0')))
        {
            let size = if ch == '\t' {
                4 - column % 4
            } else {
                ch.width().unwrap_or(0)
            };
            if column > 0 && ch != '\n' && (column >= width || column + size > width) {
                lines.push(String::new());
                column = 0;
            }
            let row = lines.len() - 1;
            positions.push((index, row, column));
            if index == self.editor.cursor {
                cursor = (column, row);
            }
            if ch == '\0' {
                break;
            }
            if ch == '\n' {
                lines.push(String::new());
                column = 0;
            } else if ch == '\t' {
                let count = size.min(width.saturating_sub(column));
                lines[row].push_str(&" ".repeat(count));
                column += count;
            } else if size <= width {
                lines[row].push(ch);
                column += size;
            }
        }
        Layout {
            lines,
            cursor,
            positions,
        }
    }
}
