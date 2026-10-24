//! Multiline prompt drafts with explicit default/custom intent and UTF-8 cursor positions.
use crate::Editor;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use engine::ai::Prompt;
use unicode_width::UnicodeWidthChar;

#[derive(Debug)]
enum Kind {
    Default,
    Custom,
}
#[derive(Debug)]
pub(crate) struct Session {
    editor: Editor,
    kind: Kind,
    pub scroll: usize,
    width: usize,
    reveal: Reveal,
}
#[derive(Debug)]
enum Reveal {
    Cursor,
    Keep,
}
pub(crate) struct Layout {
    pub lines: Vec<String>,
    pub cursor: (usize, usize),
    positions: Vec<(usize, usize, usize)>,
}
impl Session {
    pub fn new(prompt: &Prompt) -> Self {
        let mut editor = Editor::new(prompt.text(engine::commit_message::DEFAULT_PROMPT).into());
        editor.cursor = 0;
        Self {
            editor,
            kind: match prompt {
                Prompt::Default => Kind::Default,
                Prompt::Custom { .. } => Kind::Custom,
            },
            scroll: 0,
            width: 80,
            reveal: Reveal::Cursor,
        }
    }
    pub fn prompt(&self) -> Prompt {
        match self.kind {
            Kind::Default => Prompt::Default,
            Kind::Custom => Prompt::Custom {
                text: self.editor.text.clone(),
            },
        }
    }
    pub fn insert(&mut self, text: &str) -> anyhow::Result<()> {
        let text = text.replace("\r\n", "\n").replace('\r', "\n");
        anyhow::ensure!(
            text.chars()
                .all(|c| !c.is_control() || matches!(c, '\n' | '\t')),
            i18n::tr!(AgentPromptInvalid)
        );
        anyhow::ensure!(
            self.editor.text.len() + text.len() <= 16384,
            i18n::tr!(AgentPromptInvalid)
        );
        if !text.is_empty() {
            self.editor.text.insert_str(self.editor.cursor, &text);
            self.editor.cursor += text.len();
            self.kind = Kind::Custom;
            self.reveal = Reveal::Cursor;
        }
        Ok(())
    }
    pub fn key(&mut self, key: KeyEvent) -> anyhow::Result<()> {
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
                let before = self.editor.text.clone();
                self.editor.key(key);
                if self.editor.text != before {
                    self.kind = Kind::Custom;
                }
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

#[cfg(test)]
mod tests {
    use super::*;
    fn press(session: &mut Session, code: KeyCode) {
        session
            .key(KeyEvent::new(code, KeyModifiers::NONE))
            .unwrap();
    }
    #[test]
    fn chinese_multiline_navigation_survives_wrapping_resize_and_newline_at_full_width() {
        let prompt = Prompt::Custom {
            text: "中文ab\n末行".into(),
        };
        let mut session = Session::new(&prompt);
        let layout = session.view(6, 3);
        assert_eq!(layout.lines, ["中文ab", "末行"]);
        press(&mut session, KeyCode::Down);
        assert_eq!(session.view(6, 3).cursor, (0, 1));
        press(&mut session, KeyCode::End);
        session.insert("！").unwrap();
        let layout = session.view(4, 2);
        assert_eq!(layout.lines, ["中文", "ab", "末行", "！"]);
        assert_eq!(session.scroll, 2);
        assert_eq!(layout.cursor, (2, 3));
        assert_eq!(
            session.prompt(),
            Prompt::Custom {
                text: "中文ab\n末行！".into()
            }
        );
        press(&mut session, KeyCode::Backspace);
        session.view(6, 3);
        press(&mut session, KeyCode::Home);
        session.insert("新").unwrap();
        assert_eq!(
            session.prompt(),
            Prompt::Custom {
                text: "中文ab\n新末行".into()
            }
        );
    }
    #[test]
    fn oversized_or_control_paste_is_rejected_atomically() {
        let prompt = Prompt::Custom {
            text: "保留".into(),
        };
        let mut session = Session::new(&prompt);
        for text in ["\x1b[31m".to_owned(), "a".repeat(16385)] {
            assert!(session.insert(&text).is_err());
            assert_eq!(session.prompt(), prompt);
        }
        session.insert("一\r\n二\r三\t").unwrap();
        assert_eq!(
            session.prompt(),
            Prompt::Custom {
                text: "一\n二\n三\t保留".into()
            }
        );
    }
}
