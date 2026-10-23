//! A locked context token and its editable suffix have separate ownership.
use super::{Command, Context};
use crate::Editor;
use crossterm::event::{KeyCode, KeyEvent};

#[derive(Debug, Clone)]
pub(crate) enum Input {
    Global(Editor),
    Scoped { context: Context, editor: Editor },
    Prefix(Context),
}
impl Input {
    pub fn new(context: Option<Context>) -> Self {
        match context {
            Some(context) => Self::Scoped {
                context,
                editor: Editor::default(),
            },
            None => Self::global(Editor::new("/".into())),
        }
    }
    pub fn global(editor: Editor) -> Self {
        Self::Global(editor)
    }
    pub fn from_canonical(context: Option<Context>, editor: Editor) -> Self {
        let mut input = Self::new(context);
        input.replace_canonical(editor);
        input
    }
    pub fn context(&self) -> Option<Context> {
        match self {
            Self::Global(_) => None,
            Self::Scoped { context, .. } | Self::Prefix(context) => Some(*context),
        }
    }
    pub fn editor(&self) -> &Editor {
        // 前缀被选中时没有可编辑后缀，避免把锁定内容放入 Editor。
        static EMPTY: Editor = Editor {
            text: String::new(),
            cursor: 0,
        };
        match self {
            Self::Global(editor) | Self::Scoped { editor, .. } => editor,
            Self::Prefix(_) => &EMPTY,
        }
    }
    pub fn editor_mut(&mut self) -> &mut Editor {
        if let Self::Prefix(context) = self {
            *self = Self::new(Some(*context));
        }
        match self {
            Self::Global(editor) | Self::Scoped { editor, .. } => editor,
            Self::Prefix(_) => unreachable!(),
        }
    }
    pub fn prefix_selected(&self) -> bool {
        matches!(self, Self::Prefix(_))
    }
    pub fn canonical_editor(&self) -> Editor {
        match self.context() {
            None => self.editor().clone(),
            Some(context) => {
                let prefix = format!("/{} ", context.name());
                Editor {
                    text: format!("{prefix}{}", self.editor().text),
                    cursor: prefix.len() + self.editor().cursor,
                }
            }
        }
    }
    pub fn text(&self) -> String {
        self.canonical_editor().text
    }
    pub fn query(&self) -> String {
        self.text()
            .strip_prefix('/')
            .expect("command slash")
            .to_owned()
    }
    pub fn matches(&self) -> Vec<&'static Command> {
        match self.context() {
            Some(context) => context.matches(&self.editor().text),
            None => super::matches(&self.query()),
        }
    }
    pub fn name<'a>(&self, command: &'a Command) -> &'a str {
        match self.context() {
            Some(context) => context
                .relative(command)
                .expect("candidate in current group"),
            None => command.name,
        }
    }
    pub fn usage(&self, command: &Command) -> String {
        if self.context().is_none() {
            return command.usage();
        }
        match command.action.argument() {
            Some(argument) => format!("{} {}", self.name(command), argument.placeholder()),
            None => self.name(command).into(),
        }
    }
    pub fn replace_canonical(&mut self, editor: Editor) {
        match self.context() {
            None => *self = Self::Global(editor),
            Some(context) => {
                let prefix = format!("/{} ", context.name());
                let body = editor
                    .text
                    .strip_prefix(&prefix)
                    .expect("canonical completion stays in its group")
                    .to_owned();
                *self = Self::Scoped {
                    context,
                    editor: Editor {
                        text: body,
                        cursor: editor.cursor.saturating_sub(prefix.len()),
                    },
                };
            }
        }
    }
    pub fn complete(&mut self, command: &Command) {
        self.replace_canonical(Editor::new(command.completion()));
    }
    pub fn paste(&mut self, text: &str) {
        let absolute = text.strip_prefix("//").or_else(|| text.strip_prefix('/'));
        if self.editor().text.is_empty()
            && let Some(global) = absolute
        {
            *self = Self::global(Editor::new(format!("/{global}")));
            return;
        }
        if matches!(self, Self::Global(editor) if editor.text == "/")
            && let Some(text) = absolute
        {
            self.editor_mut().insert(text);
            return;
        }
        self.editor_mut().insert(text);
    }
    pub fn paste_initial(&mut self, text: &str) {
        if let Some(context) = self.context() {
            if text == context.name() {
                return;
            }
            if let Some(body) = text.strip_prefix(&format!("{} ", context.name())) {
                self.editor_mut().insert(body);
                return;
            }
        }
        self.paste(text);
    }
    pub fn namespace_key(&mut self, key: KeyEvent) -> bool {
        if !key.modifiers.is_empty() {
            return false;
        }
        if let Self::Prefix(context) = self {
            if key.code == KeyCode::Backspace || key.code == KeyCode::Char('/') {
                *self = Self::new(None);
                return true;
            }
            if key.code != KeyCode::Esc {
                *self = Self::new(Some(*context));
                return !matches!(key.code, KeyCode::Char(_));
            }
        }
        match (self.context(), key.code) {
            (Some(context), KeyCode::Backspace) if self.editor().text.is_empty() => {
                *self = Self::Prefix(context);
                true
            }
            (_, KeyCode::Char('/'))
                if self.editor().text.is_empty()
                    || matches!(self,Self::Global(editor) if editor.text == "/") =>
            {
                *self = Self::new(None);
                true
            }
            _ => false,
        }
    }
    pub fn into_search(self) -> Result<Editor, Self> {
        match self {
            Self::Global(editor) if !editor.text.starts_with('/') => Ok(editor),
            input => Err(input),
        }
    }
}
