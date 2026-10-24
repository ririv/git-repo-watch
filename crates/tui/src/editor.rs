//! UTF-8-safe single-line input shared by commands and forms.
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthChar;
#[cfg(test)]
mod tests;
#[derive(Debug, Clone, Default)]
pub struct Editor {
    pub text: String,
    pub cursor: usize,
}
impl Editor {
    pub fn new(value: String) -> Self {
        let cursor = value.len();
        Self {
            text: value,
            cursor,
        }
    }
    pub fn insert(&mut self, value: &str) {
        let value = value
            .chars()
            .filter(|c| !c.is_control())
            .collect::<String>();
        self.text.insert_str(self.cursor, &value);
        self.cursor += value.len();
    }
    pub fn key(&mut self, key: KeyEvent) {
        match (key.code, key.modifiers) {
            (KeyCode::Char('b') | KeyCode::Left, KeyModifiers::ALT) => {
                self.cursor = self.text[..self.cursor]
                    .unicode_word_indices()
                    .next_back()
                    .map_or(0, |(index, _)| index);
            }
            (KeyCode::Char('f') | KeyCode::Right, KeyModifiers::ALT) => {
                self.cursor = self.next_word_end();
            }
            (KeyCode::Char('d'), KeyModifiers::ALT) => {
                let end = self.next_word_end();
                self.text.drain(self.cursor..end);
            }
            (KeyCode::Char('a'), m) if m.contains(KeyModifiers::CONTROL) => self.cursor = 0,
            (KeyCode::Char('e'), m) if m.contains(KeyModifiers::CONTROL) => {
                self.cursor = self.text.len()
            }
            (KeyCode::Char('u'), m) if m.contains(KeyModifiers::CONTROL) => {
                self.text.drain(..self.cursor);
                self.cursor = 0;
            }
            (KeyCode::Char('w'), m) if m.contains(KeyModifiers::CONTROL) => {
                let end = self.cursor;
                while self.cursor > 0
                    && self.text[..self.cursor]
                        .chars()
                        .last()
                        .is_some_and(char::is_whitespace)
                {
                    self.left()
                }
                while self.cursor > 0
                    && self.text[..self.cursor]
                        .chars()
                        .last()
                        .is_some_and(|c| !c.is_whitespace())
                {
                    self.left()
                }
                self.text.drain(self.cursor..end);
            }
            (KeyCode::Char(c), m) if !m.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) => {
                self.insert(&c.to_string())
            }
            (KeyCode::Left, _) => self.left(),
            (KeyCode::Right, _) => self.right(),
            (KeyCode::Home, _) => self.cursor = 0,
            (KeyCode::End, _) => self.cursor = self.text.len(),
            (KeyCode::Backspace, _) if self.cursor > 0 => {
                let end = self.cursor;
                self.left();
                self.text.drain(self.cursor..end);
            }
            (KeyCode::Delete, _) if self.cursor < self.text.len() => {
                let start = self.cursor;
                self.right();
                let end = self.cursor;
                self.text.drain(start..end);
                self.cursor = start;
            }
            _ => {}
        }
    }
    fn next_word_end(&self) -> usize {
        self.text[self.cursor..]
            .unicode_word_indices()
            .next()
            .map_or(self.text.len(), |(index, word)| {
                self.cursor + index + word.len()
            })
    }
    fn left(&mut self) {
        if let Some((index, _)) = self.text[..self.cursor].char_indices().last() {
            self.cursor = index
        }
    }
    fn right(&mut self) {
        if let Some(c) = self.text[self.cursor..].chars().next() {
            self.cursor += c.len_utf8()
        }
    }
    pub fn viewport(&self, width: usize) -> (String, u16) {
        let before = &self.text[..self.cursor];
        let mut kept = Vec::new();
        let mut used = 0;
        for c in before.chars().rev() {
            let w = c.width().unwrap_or(0);
            if used + w + 1 > width {
                break;
            }
            kept.push(c);
            used += w;
        }
        kept.reverse();
        let mut out = kept.into_iter().collect::<String>();
        let cursor_column = used as u16;
        used += 1;
        for c in self.text[self.cursor..].chars() {
            let w = c.width().unwrap_or(0);
            if used + w > width {
                break;
            }
            out.push(c);
            used += w;
        }
        (out, cursor_column)
    }
}
