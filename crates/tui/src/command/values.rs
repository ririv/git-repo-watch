//! Repository parameter choices retain stable tokens independently of their labels.
use super::{ArgumentAction, Input};
use crate::Editor;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
    Branch,
    Stash,
}
impl Kind {
    pub fn for_argument(argument: ArgumentAction) -> Option<Self> {
        match argument {
            ArgumentAction::BranchSwitch => Some(Self::Branch),
            ArgumentAction::StashApply | ArgumentAction::StashPop | ArgumentAction::StashDrop => {
                Some(Self::Stash)
            }
            _ => None,
        }
    }
}
#[derive(Debug, Clone)]
pub(crate) struct Choice {
    pub token: String,
    pub label: String,
    pub description: String,
}
#[derive(Debug)]
pub(crate) struct Candidates {
    pub kind: Kind,
    entries: Arc<[Choice]>,
    pub options: Vec<usize>,
    pub selected: Option<usize>,
    command: String,
}
impl Candidates {
    pub fn new(kind: Kind, entries: Vec<Choice>, input: &Input) -> Option<Self> {
        let mut values = Self {
            kind,
            entries: entries.into(),
            options: Vec::new(),
            selected: None,
            command: String::new(),
        };
        values.refine(input)?;
        Some(values)
    }
    pub fn refine(&mut self, input: &Input) -> Option<()> {
        let editor = input.canonical_editor();
        let (argument, raw) = super::argument_input(editor.text.strip_prefix('/')?)?;
        if Kind::for_argument(argument) != Some(self.kind) {
            return None;
        }
        let start = editor.text.len() - raw.len();
        let needle = raw
            .get(..editor.cursor.checked_sub(start)?)?
            .trim()
            .to_lowercase();
        let previous = self
            .selected
            .and_then(|index| self.options.get(index))
            .copied();
        self.options = self
            .entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| {
                entry.token.to_lowercase().contains(&needle)
                    || entry.label.to_lowercase().contains(&needle)
                    || entry.description.to_lowercase().contains(&needle)
            })
            .map(|(index, _)| index)
            .collect();
        self.selected = previous
            .and_then(|entry| self.options.iter().position(|index| *index == entry))
            .or_else(|| (!self.options.is_empty()).then_some(0));
        self.command = editor.text[..start].into();
        Some(())
    }
    pub fn entries(&self) -> impl Iterator<Item = &Choice> {
        self.options.iter().map(|index| &self.entries[*index])
    }
    pub fn select(&mut self, index: usize) -> bool {
        if index >= self.options.len() {
            return false;
        }
        self.selected = Some(index);
        true
    }
    pub fn advance(&mut self, backwards: bool) {
        if let Some(index) = self.selected {
            self.selected = Some(
                (index + if backwards { self.options.len() - 1 } else { 1 }) % self.options.len(),
            );
        }
    }
    pub fn choose(&self) -> Option<Editor> {
        let entry = &self.entries[*self.options.get(self.selected?)?];
        Some(Editor::new(format!("{}{}", self.command, entry.token)))
    }
}
