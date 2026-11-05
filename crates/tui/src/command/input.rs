//! A locked context token and its editable suffix have separate ownership.
use super::{Command, Context, prefix};
use crate::Editor;
use crossterm::event::{KeyCode, KeyEvent};

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum NamespaceEvent {
    Handled,
    Pass,
    Close,
}

#[derive(Debug, Clone)]
pub(crate) enum Input {
    Global(Editor),
    Scoped { context: Context, editor: Editor },
    Prefix(prefix::Selection),
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
            Self::Scoped { context, .. } => Some(*context),
            Self::Prefix(selection) => Some(selection.origin()),
        }
    }
    pub fn menu_context(&self) -> Option<Context> {
        match self {
            Self::Scoped { context, .. } => Some(*context),
            Self::Prefix(selection) => selection.parent(),
            Self::Global(_) => None,
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
        if let Self::Prefix(selection) = self {
            *self = Self::new(Some(selection.origin()));
        }
        match self {
            Self::Global(editor) | Self::Scoped { editor, .. } => editor,
            Self::Prefix(_) => unreachable!(),
        }
    }
    pub fn prefix_selected(&self) -> bool {
        matches!(self, Self::Prefix(_))
    }
    pub fn prefix_selection(&self) -> Option<std::ops::Range<usize>> {
        match self {
            Self::Prefix(selection) => Some(selection.range()),
            Self::Global(_) | Self::Scoped { .. } => None,
        }
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
        if let Self::Prefix(selection) = self {
            return selection
                .parent()
                .map(|parent| format!("{} ", parent.name()))
                .unwrap_or_default();
        }
        self.text()
            .strip_prefix('/')
            .expect("command slash")
            .to_owned()
    }
    pub fn matches(&self) -> Vec<&'static Command> {
        match self.menu_context() {
            Some(context) => context.matches(&self.editor().text),
            None => super::matches(&self.query()),
        }
    }
    pub fn name<'a>(&self, command: &'a Command) -> &'a str {
        match self.menu_context() {
            Some(_) if command.common() => command.name,
            Some(context) => context
                .relative(command)
                .expect("candidate in current group"),
            None => command.name,
        }
    }
    pub fn usage(&self, command: &Command) -> String {
        if self.menu_context().is_none() || command.common() {
            return command.usage();
        }
        match command.action.argument() {
            Some(argument) => format!("{} {}", self.name(command), argument.placeholder()),
            None => self.name(command).into(),
        }
    }
    pub fn replace_canonical(&mut self, editor: Editor) {
        match self.menu_context() {
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
        if command.common() {
            *self = Self::global(Editor::new(command.completion()));
            return;
        }
        self.replace_canonical(Editor::new(command.completion()));
    }
    pub fn paste(&mut self, text: &str) {
        if let Self::Prefix(selection) = self {
            // 粘贴与键入采用同一选区语义；空粘贴保留选区，控制字符不进入命令。
            let text = text.chars().filter(|c| !c.is_control()).collect::<String>();
            if !text.is_empty() {
                *self = Self::new(selection.parent());
                self.paste(&text);
            }
            return;
        }
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
    pub fn namespace_key(&mut self, key: KeyEvent) -> NamespaceEvent {
        if self.prefix_selected()
            && matches!(
                key.modifiers,
                crossterm::event::KeyModifiers::NONE | crossterm::event::KeyModifiers::SHIFT
            )
            && let KeyCode::Char(character) = key.code
        {
            let Self::Prefix(selection) = self else {
                unreachable!()
            };
            // 保留未选中的父级；显式输入 / 则直接切换到全局命令。
            *self = Self::new(if character == '/' {
                None
            } else {
                selection.parent()
            });
            if character != '/' {
                self.editor_mut().key(key);
            }
            return NamespaceEvent::Handled;
        }
        if !key.modifiers.is_empty() {
            return NamespaceEvent::Pass;
        }
        if let Self::Prefix(selection) = self {
            match key.code {
                KeyCode::Left => selection.expand(),
                KeyCode::Right if selection.shrink() => {}
                KeyCode::Backspace | KeyCode::Delete => match selection.parent() {
                    Some(parent) => *self = Self::new(Some(parent)),
                    None => return NamespaceEvent::Close,
                },
                KeyCode::Up | KeyCode::Down | KeyCode::Tab | KeyCode::BackTab => {
                    return NamespaceEvent::Pass;
                }
                _ => *self = Self::new(Some(selection.origin())),
            }
            return NamespaceEvent::Handled;
        }
        match (self.context(), key.code) {
            (Some(context), KeyCode::Backspace) if self.editor().text.is_empty() => {
                *self = Self::Prefix(prefix::Selection::last(context));
                NamespaceEvent::Handled
            }
            (_, KeyCode::Char('/'))
                if self.editor().text.is_empty()
                    || matches!(self,Self::Global(editor) if editor.text == "/") =>
            {
                *self = Self::new(None);
                NamespaceEvent::Handled
            }
            _ => NamespaceEvent::Pass,
        }
    }
    pub fn into_search(self) -> Result<Editor, Self> {
        match self {
            Self::Global(editor) if !editor.text.starts_with('/') => Ok(editor),
            input => Err(input),
        }
    }
}
