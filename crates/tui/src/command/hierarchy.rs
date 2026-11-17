//! Command hierarchy is derived from visible candidates and shared by rendering and navigation.
mod tree;
use super::{Command, Input};
pub(crate) use tree::{Label, Tree, arrange};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Location {
    Parent,
    Children,
}
pub(crate) struct Group {
    pub parent: &'static Command,
    pub location: Location,
}
fn descendant(name: &str, parent: &str) -> bool {
    name.strip_prefix(parent)
        .is_some_and(|tail| tail.starts_with(' '))
}
pub(crate) fn has_children(command: &Command, candidates: &[&Command]) -> bool {
    command.action.argument().is_none()
        && candidates
            .iter()
            .any(|child| descendant(child.name, command.name))
}
impl Group {
    pub fn current(input: &Input, candidates: &[&'static Command]) -> Option<Self> {
        if input.prefix_selected() || super::argument_input(&input.query()).is_some() {
            return None;
        }
        let query = input.query().trim_start().to_ascii_lowercase();
        let parent = super::COMMANDS
            .iter()
            .filter(|command| {
                input
                    .menu_context()
                    .is_none_or(|context| context.admits(command) || command.common())
            })
            .filter(|command| query.trim_end() == command.name)
            .filter(|command| has_children(command, candidates))
            .max_by_key(|command| command.name.len())?;
        Some(Self {
            parent,
            location: if query == parent.name {
                Location::Parent
            } else {
                Location::Children
            },
        })
    }
    pub fn retain_candidates(&self, candidates: &mut Vec<&'static Command>) {
        candidates.retain(|command| {
            command.name == self.parent.name || descendant(command.name, self.parent.name)
        });
    }
    pub fn title(&self) -> Option<String> {
        (self.location == Location::Children)
            .then(|| i18n::tf!(CommandChildrenTitle, name = self.parent.name))
    }
}
