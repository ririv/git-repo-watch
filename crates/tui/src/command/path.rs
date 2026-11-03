//! Literal directory completion at the UTF-8 cursor; never evaluates shell syntax.
use crate::Editor;
use anyhow::{Context, Result, ensure};
use std::{
    fs,
    path::{Path, PathBuf},
};

use super::completion::Menu;

#[derive(Debug)]
pub(crate) struct Directories {
    input: Box<PathInput>,
    available: Vec<String>,
    base_prefix: String,
    names: Vec<String>,
    selected: usize,
}
impl Directories {
    pub fn names(&self) -> &[String] {
        &self.names
    }
    pub fn selected(&self) -> usize {
        self.selected
    }
    pub fn select(&mut self, index: usize) -> bool {
        if index >= self.names.len() {
            return false;
        }
        self.selected = index;
        true
    }
    pub fn choose(&self) -> Option<Editor> {
        Some(self.input.replace(self.names.get(self.selected)?, true))
    }
    pub fn refine(&mut self, editor: &Editor, home: &Path) -> Result<bool> {
        let Some(input) = PathInput::at(editor, home)? else {
            return Ok(false);
        };
        if input.directory != self.input.directory || !input.prefix.starts_with(&self.base_prefix) {
            return Ok(false);
        }
        self.names = self
            .available
            .iter()
            .filter(|name| name.starts_with(&input.prefix))
            .cloned()
            .collect();
        self.selected = 0;
        *self.input = input;
        Ok(true)
    }
    pub fn advance(&mut self, backwards: bool) {
        if self.names.is_empty() {
            return;
        }
        self.selected = if backwards {
            (self.selected + self.names.len() - 1) % self.names.len()
        } else {
            (self.selected + 1) % self.names.len()
        };
    }
}

#[derive(Debug)]
enum Format {
    Literal,
    Token,
}
#[derive(Debug)]
struct PathInput {
    command: String,
    quote: Option<char>,
    parent: String,
    directory: PathBuf,
    prefix: String,
    suffix: String,
    trailing: String,
    format: Format,
}
impl PathInput {
    fn at(editor: &Editor, user_home: &Path) -> Result<Option<Self>> {
        let Some((argument, raw)) =
            super::argument_input(editor.text.strip_prefix('/').expect("command prefix"))
        else {
            return Ok(None);
        };
        if argument == super::ArgumentAction::Scan {
            return Self::scan(editor, user_home, raw);
        }
        if !argument.is_path() {
            return Ok(None);
        }
        let raw_start = editor.text.len() - raw.len();
        let value = raw.trim_start();
        let start = raw_start + raw.len() - value.len();
        let quote = value.chars().next().filter(|c| matches!(c, '\'' | '"'));
        let value = if quote.is_some_and(|q| value.len() > 1 && value.trim_end().ends_with(q)) {
            value.trim_end()
        } else {
            value
        };
        let content_start = start + usize::from(quote.is_some());
        let content = &value[usize::from(quote.is_some())..];
        let content = if quote.is_some_and(|q| content.ends_with(q)) {
            &content[..content.len() - 1]
        } else {
            content
        };
        if editor.cursor < content_start {
            return Ok(None);
        }
        let position = (editor.cursor - content_start).min(content.len());
        // 空参数明确从文件系统根目录开始，不猜测某个仓库或进程目录。
        let (content, position) = if content.is_empty() {
            ("/", 1)
        } else if content == "~" && position == 1 {
            ("~/", 2)
        } else {
            (content, position)
        };
        if position == 0 || (content.starts_with("~/") && position == 1) {
            return Ok(None);
        }
        ensure!(
            engine::paths::expand_home(content, user_home)?.is_absolute(),
            i18n::tr!(EnterAnAbsolutePath)
        );
        let component_start = content[..position].rfind('/').map_or(0, |i| i + 1);
        let component_end = content[position..]
            .find('/')
            .map_or(content.len(), |i| position + i);
        Ok(Some(Self {
            command: editor.text[..start].into(),
            quote,
            parent: content[..component_start].into(),
            directory: engine::paths::expand_home(&content[..component_start], user_home)?,
            prefix: content[component_start..position].into(),
            suffix: content[component_end..].into(),
            trailing: String::new(),
            format: Format::Literal,
        }))
    }
    fn scan(editor: &Editor, home: &Path, raw: &str) -> Result<Option<Self>> {
        let start = editor.text.len() - raw.len();
        if editor.cursor < start {
            return Ok(None);
        }
        let position = editor.cursor - start;
        let tokens = super::scan::tokens(raw);
        let active = tokens
            .iter()
            .position(|token| token.range.start <= position && position <= token.range.end);
        let index = active.unwrap_or(
            tokens
                .iter()
                .take_while(|token| token.range.end < position)
                .count(),
        );
        if index == 1 || (index > 1 && tokens.get(1).is_none_or(|token| token.value != "--exclude"))
        {
            return Ok(None);
        }
        let (range, content, position, quote) = if let Some(index) = active {
            let token = &tokens[index];
            let prefix = super::scan::tokens(&raw[token.range.start..position]);
            (
                token.range.clone(),
                token.value.clone(),
                prefix.last().map_or(0, |token| token.value.len()),
                raw[token.range.clone()]
                    .chars()
                    .next()
                    .filter(|ch| matches!(ch, '\'' | '"')),
            )
        } else {
            (position..position, String::new(), 0, None)
        };
        let root = if index == 0 {
            PathBuf::from("/")
        } else {
            let root = engine::paths::expand_home(&tokens[0].value, home)?;
            ensure!(root.is_absolute(), i18n::tr!(EnterAnAbsolutePath));
            root
        };
        let (content, position) = if content.is_empty() && index == 0 {
            ("/".to_owned(), 1)
        } else if content == "~" && position == 1 {
            ("~/".to_owned(), 2)
        } else {
            (content, position)
        };
        if index == 0 {
            ensure!(
                engine::paths::expand_home(&content, home)?.is_absolute(),
                i18n::tr!(EnterAnAbsolutePath)
            );
        }
        let component_start = content[..position].rfind('/').map_or(0, |i| i + 1);
        let component_end = content[position..]
            .find('/')
            .map_or(content.len(), |i| position + i);
        let parent = &content[..component_start];
        let expanded = engine::paths::expand_home(parent, home)?;
        let directory = if expanded.is_absolute() {
            expanded
        } else {
            root.join(expanded)
        };
        Ok(Some(Self {
            command: editor.text[..start + range.start].into(),
            quote,
            parent: parent.into(),
            directory,
            prefix: content[component_start..position].into(),
            suffix: content[component_end..].into(),
            trailing: editor.text[start + range.end..].into(),
            format: Format::Token,
        }))
    }
    fn replace(&self, name: &str, directory: bool) -> Editor {
        let mut value = format!("{}{name}", self.parent);
        let mut position = value.len();
        if directory {
            if !self.suffix.starts_with('/') {
                value.push('/');
            }
            position += 1;
        }
        value.push_str(&self.suffix);
        if matches!(self.format, Format::Token) {
            let (value, position) = super::scan::encode(&value, position, self.quote);
            return Editor {
                cursor: self.command.len() + position,
                text: format!("{}{}{}", self.command, value, self.trailing),
            };
        }
        let quote = self.quote.map(String::from).unwrap_or_default();
        Editor {
            cursor: self.command.len() + quote.len() + position,
            text: format!("{}{quote}{value}{quote}", self.command),
        }
    }
}

/// Completes a single directory component and returns a menu only for ambiguous paths.
#[cfg(test)]
pub(crate) fn complete(editor: &mut Editor, user_home: &Path) -> Result<Menu> {
    complete_cancellable(editor, user_home, &engine::cancel::Cancel::default())
}
pub(crate) fn complete_cancellable(
    editor: &mut Editor,
    user_home: &Path,
    cancel: &engine::cancel::Cancel,
) -> Result<Menu> {
    cancel.check()?;
    if complete_scan_flag(editor) {
        return Ok(Menu::Commands);
    }
    let Some(input) = PathInput::at(editor, user_home)? else {
        return Ok(Menu::Commands);
    };
    let mut names: Vec<String> = Vec::new();
    for entry in fs::read_dir(&input.directory)
        .with_context(|| i18n::tf!(ReadCompletionDirectory, &input.parent))?
    {
        cancel.check()?;
        let entry = entry.with_context(|| i18n::tf!(ReadCompletionDirectory, &input.parent))?;
        let name = entry.file_name();
        // 非 UTF-8 名称无法在输入栏中无损编辑，匹配时明确拒绝，避免有损转换。
        if !name.as_encoded_bytes().starts_with(input.prefix.as_bytes()) {
            continue;
        }
        if name.as_encoded_bytes().starts_with(b".") && !input.prefix.starts_with('.') {
            continue;
        }
        let kind = entry
            .file_type()
            .with_context(|| i18n::tf!(ReadCompletionDirectory, &input.parent))?;
        let is_directory = if kind.is_symlink() {
            match entry.path().metadata() {
                Ok(metadata) => metadata.is_dir(),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
                Err(error) => {
                    return Err(error)
                        .with_context(|| i18n::tf!(ReadCompletionDirectory, &input.parent));
                }
            }
        } else {
            kind.is_dir()
        };
        if !is_directory {
            continue;
        }
        let name = name.to_str().context(i18n::tr!(UneditableDirectoryName))?;
        ensure!(
            !name.chars().any(char::is_control),
            i18n::tr!(UneditableDirectoryName)
        );
        names.push(name.into());
    }
    names.sort();
    ensure!(!names.is_empty(), i18n::tr!(NoMatchingDirectories));
    if names.len() == 1 {
        *editor = input.replace(&names[0], true);
        return Ok(Menu::Commands);
    }
    let common = names[0]
        .chars()
        .take_while({
            let mut position = 0;
            let names = &names;
            move |c| {
                let matches = names.iter().all(|name| name[position..].starts_with(*c));
                if matches {
                    position += c.len_utf8();
                }
                matches
            }
        })
        .collect::<String>();
    *editor = input.replace(&common, false);
    Ok(Menu::Directories(Directories {
        base_prefix: input.prefix.clone(),
        input: Box::new(input),
        available: names.clone(),
        names,
        selected: 0,
    }))
}

#[cfg(test)]
mod tests;

fn complete_scan_flag(editor: &mut Editor) -> bool {
    let Some((super::ArgumentAction::Scan, raw)) =
        super::argument_input(editor.text.strip_prefix('/').expect("command slash"))
    else {
        return false;
    };
    let start = editor.text.len() - raw.len();
    if editor.cursor < start {
        return false;
    }
    let position = editor.cursor - start;
    let tokens = super::scan::tokens(raw);
    if tokens.is_empty() || position <= tokens[0].range.end {
        return false;
    }
    let token = tokens.get(1);
    let (range, value) = match token {
        Some(token) if token.range.start <= position && position <= token.range.end => {
            (token.range.clone(), &token.value[..])
        }
        None => (position..position, ""),
        _ => return false,
    };
    if !"--exclude".starts_with(value) {
        return false;
    }
    let command = format!(
        "{}--exclude {}",
        &editor.text[..start + range.start],
        &editor.text[start + range.end..]
    );
    editor.cursor = start + range.start + "--exclude ".len();
    editor.text = command;
    true
}
