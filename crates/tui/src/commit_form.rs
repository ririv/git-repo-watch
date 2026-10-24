//! Commit text ownership separates editable drafts from read-only generation previews.
use crate::multiline_editor::Editor;
use engine::commit_message::{MESSAGE_LIMIT, Update};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EditAction {
    Commit,
    Return,
    Regenerate,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Action {
    Edit(EditAction),
    CancelGeneration,
}
impl Action {
    pub fn label(self) -> &'static str {
        match self {
            Self::Edit(EditAction::Commit) => i18n::tr!(ActionCommit),
            Self::Edit(EditAction::Regenerate) => i18n::tr!(CommitRegenerateAction),
            Self::Edit(EditAction::Return) => i18n::tr!(CommitReturnAction),
            Self::CancelGeneration => i18n::tr!(CommitCancelAction),
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Focus {
    Message,
    Actions(EditAction),
}
#[derive(Debug)]
pub(crate) enum Draft {
    Editing { editor: Editor, focus: Focus },
    Generating { original: Editor, preview: Editor },
    Submitting { editor: Editor },
}
impl Draft {
    pub fn new(message: String) -> Self {
        Self::Editing {
            editor: Editor::new(message, MESSAGE_LIMIT),
            focus: Focus::Message,
        }
    }
    pub fn text(&self) -> &str {
        match self {
            Self::Editing { editor, .. } | Self::Submitting { editor } => editor.text(),
            Self::Generating { preview, .. } => preview.text(),
        }
    }
    pub fn focused(&self) -> bool {
        matches!(
            self,
            Self::Editing {
                focus: Focus::Message,
                ..
            }
        )
    }
    pub fn generating(&self) -> bool {
        matches!(self, Self::Generating { .. })
    }
    pub fn submitting(&self) -> bool {
        matches!(self, Self::Submitting { .. })
    }
    pub fn focus(&mut self, next: Focus) {
        if let Self::Editing { focus, .. } = self {
            *focus = next;
        }
    }
    pub fn actions(&self, ai: bool) -> &'static [Action] {
        use Action::{CancelGeneration, Edit};
        use EditAction::{Commit, Regenerate, Return};
        match self {
            Self::Generating { .. } => &[CancelGeneration],
            Self::Submitting { .. } => &[Edit(Return)],
            Self::Editing { .. } if ai => &[Edit(Commit), Edit(Return), Edit(Regenerate)],
            Self::Editing { .. } => &[Edit(Commit), Edit(Return)],
        }
    }
    pub fn selected(&self) -> Option<Action> {
        match self {
            Self::Editing {
                focus: Focus::Message,
                ..
            } => None,
            Self::Editing {
                focus: Focus::Actions(action),
                ..
            } => Some(Action::Edit(*action)),
            Self::Generating { .. } => Some(Action::CancelGeneration),
            Self::Submitting { .. } => Some(Action::Edit(EditAction::Return)),
        }
    }
    pub fn select(&mut self, action: Action) {
        if let Action::Edit(action) = action {
            self.focus(Focus::Actions(action));
        }
    }
    pub fn tab(&mut self, backwards: bool, ai: bool) {
        let actions = self.actions(ai);
        let current = self
            .selected()
            .and_then(|selected| actions.iter().position(|action| *action == selected));
        let next = match (current, backwards) {
            (None, false) => Some(0),
            (None, true) => Some(actions.len() - 1),
            (Some(0), true) => None,
            (Some(index), false) if index + 1 == actions.len() => None,
            (Some(index), true) => Some(index - 1),
            (Some(index), false) => Some(index + 1),
        };
        if let Some(index) = next {
            self.select(actions[index]);
        } else {
            self.focus(Focus::Message);
        }
    }
    pub fn step(&mut self, backwards: bool, ai: bool) {
        let actions = self.actions(ai);
        if let Some(index) = self
            .selected()
            .and_then(|selected| actions.iter().position(|action| *action == selected))
        {
            let next = if backwards {
                (index + actions.len() - 1) % actions.len()
            } else {
                (index + 1) % actions.len()
            };
            self.select(actions[next]);
        }
    }
    pub fn editable(&mut self) -> Option<&mut Editor> {
        match self {
            Self::Editing { editor, .. } => Some(editor),
            Self::Generating { .. } | Self::Submitting { .. } => None,
        }
    }
    pub fn display(&mut self) -> &mut Editor {
        match self {
            Self::Editing { editor, .. } | Self::Submitting { editor } => editor,
            Self::Generating { preview, .. } => preview,
        }
    }
    pub fn begin(&mut self) {
        let current = std::mem::replace(self, Self::new(String::new()));
        *self = match current {
            Self::Editing { editor, .. } => Self::Generating {
                original: editor,
                preview: Editor::new(String::new(), MESSAGE_LIMIT),
            },
            generating => generating,
        };
    }
    pub fn submit(&mut self) {
        let current = std::mem::replace(self, Self::new(String::new()));
        *self = match current {
            Self::Editing { editor, .. } => Self::Submitting { editor },
            other => other,
        };
    }
    pub fn update(&mut self, update: Update) {
        if let Self::Generating { preview, .. } = self {
            match update {
                Update::Append(delta) => {
                    let mut text = preview.text().to_owned();
                    text.push_str(&delta);
                    *preview = Editor::new(text, MESSAGE_LIMIT);
                }
                Update::Replace(text) => *preview = Editor::new(text, MESSAGE_LIMIT),
            }
        }
    }
    pub fn accept(&mut self, text: String) {
        let mut next = Self::new(text);
        if next.text().contains('\n') {
            next.display().start();
        }
        *self = next;
    }
    pub fn restore(&mut self) {
        let current = std::mem::replace(self, Self::new(String::new()));
        *self = match current {
            Self::Generating { original, .. } | Self::Submitting { editor: original } => {
                Self::Editing {
                    editor: original,
                    focus: Focus::Message,
                }
            }
            editing => editing,
        };
    }
}
