//! Input follows the focused tree or text pane without sharing their drafts.
use super::{Focus, Session};
impl Session {
    pub fn content_searchable(&self) -> bool {
        !self.panes.pending()
            && self.panes.feedback.previous().is_none()
            && self.panes.get(self.presentation().active()).searchable()
    }
    pub fn input_query(&self) -> &str {
        match self.focus {
            Focus::Tree => &self.query,
            Focus::Preview => self
                .panes
                .get(self.presentation().active())
                .position
                .find
                .query(),
        }
    }
    pub fn search_input(&mut self, query: String) {
        match self.focus {
            Focus::Tree => self.search(query),
            Focus::Preview if self.content_searchable() => self.pane().search(query),
            Focus::Preview => {}
        }
    }
    pub fn next_content_match(&mut self, backwards: bool) {
        let pane = self.pane();
        pane.find.step(&mut pane.position.find, backwards);
    }
    pub fn content_match_label(&self) -> String {
        let pane = self.panes.get(self.presentation().active());
        pane.find.label(&pane.position.find)
    }
}
