//! Route horizontal scrolling without stealing editor or confirmation keys.
use crate::{Mode, Screen, bar::InputBar, changes, document, history};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

impl Screen {
    pub(super) fn horizontal_key(&mut self, key: KeyEvent) -> bool {
        if key.modifiers != KeyModifiers::NONE
            || !matches!(self.bar, InputBar::Search(_))
            || !self.bar.editor().text.is_empty()
        {
            return false;
        }
        let delta = match key.code {
            KeyCode::Left => -4,
            KeyCode::Right => 4,
            _ => return false,
        };
        let content = match &mut self.mode {
            Mode::Browser(session) if session.focus == crate::browser::Focus::Preview => {
                session.pane().position.horizontal.advance(delta);
                return true;
            }
            Mode::Changes(session) if session.focus == changes::Focus::Preview => {
                match &mut session.preview {
                    changes::Preview::Ready { content, .. } => content,
                    // 异步预览未就绪时也不把平移键解释成返回，避免时序改变按键含义。
                    _ => return true,
                }
            }
            Mode::History(view) => match view.as_mut() {
                history::View::Graph(session) => {
                    session.horizontal.advance(delta);
                    return true;
                }
                history::View::Diff { content, .. } => content,
                history::View::Commit(_) => return false,
            },
            _ => return false,
        };
        if let document::View::Text(view) = content.as_mut() {
            view.horizontal.advance(delta);
        }
        // 图片仍按窗口适配；同一预览中的左右键不会触发导航。
        true
    }
}
