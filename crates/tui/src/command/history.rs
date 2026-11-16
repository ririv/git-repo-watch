//! History is a completion source; accepting a row only fills the command draft.
use crate::Editor;

#[derive(Debug)]
pub(crate) struct Session {
    pub entries: Vec<String>,
    pub selected: usize,
}
impl Session {
    pub fn new(entries: Vec<String>) -> Self {
        Self {
            entries,
            selected: 0,
        }
    }
    pub fn select(&mut self, index: usize) -> bool {
        if index >= self.entries.len() {
            return false;
        }
        self.selected = index;
        true
    }
    pub fn choose(&self) -> Option<Editor> {
        self.entries.get(self.selected).cloned().map(Editor::new)
    }
    pub fn advance(&mut self, backwards: bool) {
        self.selected = if backwards {
            self.selected.saturating_sub(1)
        } else {
            (self.selected + 1).min(self.entries.len().saturating_sub(1))
        };
    }
}
