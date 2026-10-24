//! Complete prompt drafts preserve explicit default/custom intent.
use crate::multiline_editor;
use crossterm::event::KeyEvent;
use engine::ai::Prompt;

#[derive(Debug)]
enum Kind {
    Default,
    Custom,
}
#[derive(Debug)]
pub(crate) struct Session {
    pub editor: multiline_editor::Editor,
    kind: Kind,
}
impl Session {
    pub fn new(prompt: &Prompt) -> Self {
        let mut editor = multiline_editor::Editor::new(
            prompt.text(engine::commit_message::DEFAULT_PROMPT).into(),
            16384,
        );
        editor.start();
        Self {
            editor,
            kind: match prompt {
                Prompt::Default => Kind::Default,
                Prompt::Custom { .. } => Kind::Custom,
            },
        }
    }
    pub fn prompt(&self) -> Prompt {
        match self.kind {
            Kind::Default => Prompt::Default,
            Kind::Custom => Prompt::Custom {
                text: self.editor.text().into(),
            },
        }
    }
    pub fn insert(&mut self, text: &str) -> anyhow::Result<()> {
        self.edit(|editor| editor.insert(text))
    }
    pub fn key(&mut self, key: KeyEvent) -> anyhow::Result<()> {
        self.edit(|editor| editor.key(key))
    }
    fn edit(
        &mut self,
        edit: impl FnOnce(&mut multiline_editor::Editor) -> Result<(), multiline_editor::Invalid>,
    ) -> anyhow::Result<()> {
        let before = self.editor.text().to_owned();
        edit(&mut self.editor).map_err(|_| anyhow::anyhow!(i18n::tr!(AgentPromptInvalid)))?;
        if self.editor.text() != before {
            self.kind = Kind::Custom;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyModifiers};
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
        let layout = session.editor.view(6, 3);
        assert_eq!(layout.lines, ["中文ab", "末行"]);
        press(&mut session, KeyCode::Down);
        assert_eq!(session.editor.view(6, 3).cursor, (0, 1));
        press(&mut session, KeyCode::End);
        session.insert("！").unwrap();
        let layout = session.editor.view(4, 2);
        assert_eq!(layout.lines, ["中文", "ab", "末行", "！"]);
        assert_eq!(session.editor.scroll, 2);
        assert_eq!(layout.cursor, (2, 3));
        assert_eq!(
            session.prompt(),
            Prompt::Custom {
                text: "中文ab\n末行！".into()
            }
        );
        press(&mut session, KeyCode::Backspace);
        session.editor.view(6, 3);
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
