//! Route horizontal scrolling without stealing editor or confirmation keys.
use crate::{Mode, Screen, changes, history};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

impl Screen {
    pub(super) fn horizontal_key(&mut self, key: KeyEvent) -> bool {
        if key.modifiers != KeyModifiers::NONE
            || !self.bar.is_plain()
            || !self.bar.editor().text.is_empty()
        {
            return false;
        }
        let delta = match key.code {
            KeyCode::Left => -4,
            KeyCode::Right => 4,
            _ => return false,
        };
        self.horizontal_scroll(delta)
    }
    pub(super) fn horizontal_scroll(&mut self, delta: isize) -> bool {
        if let Some(inspect) = self.mode.inspection_mut() {
            if inspect.focus == crate::inspection::Focus::Preview {
                inspect.review.horizontal.advance(delta);
                return true;
            }
            return false;
        }
        let content = match &mut self.mode {
            Mode::Browser(session) => {
                match session.focus {
                    crate::browser::Focus::Tree => session.horizontal.advance(delta),
                    crate::browser::Focus::Preview => {
                        session.pane().position.horizontal.advance(delta)
                    }
                }
                return true;
            }
            Mode::Changes(session) if session.focus == changes::Focus::Preview => {
                match &mut session.preview {
                    changes::Preview::Ready { content, .. } => content,
                    // 异步预览未就绪时也不把平移键解释成返回，避免时序改变按键含义。
                    _ => return true,
                }
            }
            Mode::Stashes(view) => match view.as_mut() {
                crate::stashes::View::Diff { content, .. } => content,
                _ => return false,
            },
            Mode::History(view) => match view.as_mut() {
                history::View::Graph(session) => {
                    if session.focus == history::GraphFocus::Files {
                        return false;
                    }
                    session.horizontal.advance(delta);
                    return true;
                }
                history::View::Diff { content, .. } => content,
                history::View::Commit(_) => return false,
            },
            _ => return false,
        };
        if let Some(horizontal) = content.horizontal() {
            horizontal.advance(delta);
        }
        // 图片仍按窗口适配；同一预览中的左右键不会触发导航。
        true
    }
}
