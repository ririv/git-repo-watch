//! Result reading is an overlay that retains the underlying page and command draft.
use crate::{Screen, view};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::{Position, Rect};

#[derive(Debug)]
pub(crate) enum Feedback {
    Inline { expansion: Option<Rect> },
    Reading(Reader),
}
impl Default for Feedback {
    fn default() -> Self {
        Self::Inline { expansion: None }
    }
}
#[derive(Debug)]
pub(crate) struct Reader {
    pub text: String,
    pub scroll: usize,
    pub maximum: usize,
    pub content: Rect,
    pub back: Rect,
}
impl Screen {
    pub(crate) fn expand_feedback(&mut self) -> bool {
        let Feedback::Inline { expansion: Some(_) } = self.feedback else {
            return false;
        };
        let text = view::input::status_text(self, self.body_area.expect("rendered result"));
        self.feedback = Feedback::Reading(Reader {
            text,
            scroll: 0,
            maximum: 0,
            content: Rect::default(),
            back: Rect::default(),
        });
        true
    }
    pub(crate) fn feedback_key(&mut self, key: KeyEvent) -> bool {
        if key.code == KeyCode::Enter && key.modifiers == KeyModifiers::ALT {
            if matches!(self.feedback, Feedback::Reading(_)) {
                self.feedback = Feedback::default();
                return true;
            }
            self.expand_feedback();
            return true;
        }
        let Feedback::Reading(reader) = &mut self.feedback else {
            return false;
        };
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            return false;
        }
        match key.code {
            KeyCode::Esc | KeyCode::Enter => self.feedback = Feedback::default(),
            KeyCode::Up => reader.scroll = reader.scroll.saturating_sub(1),
            KeyCode::Down => reader.scroll = reader.scroll.saturating_add(1).min(reader.maximum),
            KeyCode::PageUp => {
                reader.scroll = reader
                    .scroll
                    .saturating_sub(usize::from(reader.content.height).max(1))
            }
            KeyCode::PageDown => {
                reader.scroll = reader
                    .scroll
                    .saturating_add(usize::from(reader.content.height).max(1))
                    .min(reader.maximum)
            }
            KeyCode::Home => reader.scroll = 0,
            KeyCode::End => reader.scroll = reader.maximum,
            _ => {}
        }
        true
    }
    pub(crate) fn feedback_mouse(&mut self, event: MouseEvent) -> bool {
        let point = Position::new(event.column, event.row);
        match &mut self.feedback {
            Feedback::Inline { expansion } => {
                if self.config.mouse.precise_click
                    && event.kind == MouseEventKind::Down(MouseButton::Left)
                    && expansion.is_some_and(|area| area.contains(point))
                {
                    return self.expand_feedback();
                }
                false
            }
            Feedback::Reading(reader) => {
                match event.kind {
                    MouseEventKind::Down(MouseButton::Left)
                        if self.config.mouse.precise_click && reader.back.contains(point) =>
                    {
                        self.feedback = Feedback::default();
                    }
                    MouseEventKind::ScrollUp
                        if self.config.mouse.scroll && reader.content.contains(point) =>
                    {
                        reader.scroll = reader.scroll.saturating_sub(3);
                    }
                    MouseEventKind::ScrollDown
                        if self.config.mouse.scroll && reader.content.contains(point) =>
                    {
                        reader.scroll = reader.scroll.saturating_add(3).min(reader.maximum);
                    }
                    _ => {}
                }
                true
            }
        }
    }
}
