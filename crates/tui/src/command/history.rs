//! Command history supplies canonical drafts for completion and submission.
use crate::Editor;

#[derive(Debug)]
pub(crate) struct Session {
    pub entries: Vec<String>,
    pub selected: usize,
}
impl Session {
    pub fn new(mut entries: Vec<String>) -> Self {
        // 存储按新到旧返回；菜单紧贴输入栏，从底部最近一条向上浏览。
        entries.reverse();
        let selected = entries.len().saturating_sub(1);
        Self { entries, selected }
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
