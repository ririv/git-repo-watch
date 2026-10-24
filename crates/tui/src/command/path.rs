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
    input: PathInput,
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
    pub fn choose(&self) -> Editor {
        self.input.replace(&self.names[self.selected], true)
    }
    pub fn advance(&mut self, backwards: bool) {
        self.selected = if backwards {
            (self.selected + self.names.len() - 1) % self.names.len()
        } else {
            (self.selected + 1) % self.names.len()
        };
    }
}

#[derive(Debug)]
struct PathInput {
    command: String,
    quote: Option<char>,
    parent: String,
    directory: PathBuf,
    prefix: String,
    suffix: String,
}
impl PathInput {
    fn at(editor: &Editor, user_home: &Path) -> Result<Option<Self>> {
        let Some((argument, raw)) =
            super::argument_input(editor.text.strip_prefix('/').expect("command prefix"))
        else {
            return Ok(None);
        };
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
        let quote = self.quote.map(String::from).unwrap_or_default();
        Editor {
            cursor: self.command.len() + quote.len() + position,
            text: format!("{}{quote}{value}{quote}", self.command),
        }
    }
}

/// Completes a single directory component and returns a menu only for ambiguous paths.
pub(crate) fn complete(editor: &mut Editor, user_home: &Path) -> Result<Menu> {
    let Some(input) = PathInput::at(editor, user_home)? else {
        return Ok(Menu::Commands);
    };
    let mut names: Vec<String> = Vec::new();
    for entry in fs::read_dir(&input.directory)
        .with_context(|| i18n::tf!(ReadCompletionDirectory, &input.parent))?
    {
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
        input,
        names,
        selected: 0,
    }))
}

#[cfg(test)]
mod tests;
