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
fn has_children(command: &Command, candidates: &[&Command]) -> bool {
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
        let relative = input.editor().text.trim_start().to_ascii_lowercase();
        super::COMMANDS
            .iter()
            .filter(|command| {
                input
                    .menu_context()
                    .is_none_or(|context| context.admits(command) || command.common())
            })
            .filter_map(|command| {
                let matched = if command.common() && input.menu_context().is_some() {
                    &relative
                } else {
                    &query
                };
                (matched.trim_end() == command.name
                    && (has_children(command, candidates) || super::named::has_parameters(command)))
                .then_some(Self {
                    parent: command,
                    location: if matched == command.name {
                        Location::Parent
                    } else {
                        Location::Children
                    },
                })
            })
            .max_by_key(|group| group.parent.name.len())
    }

    pub fn at_parent(&self, command: &Command) -> bool {
        self.location == Location::Parent && self.parent.name == command.name
    }

    pub fn retain_candidates(&self, candidates: &mut Vec<&'static Command>) {
        candidates.retain(|command| {
            self.at_parent(command) || descendant(command.name, self.parent.name)
        });
    }
    pub fn title(&self) -> Option<String> {
        (self.location == Location::Children).then(|| {
            if super::named::has_parameters(self.parent) {
                i18n::tf!(CommandGroupTitle, name = self.parent.name)
            } else {
                i18n::tf!(CommandChildrenTitle, name = self.parent.name)
            }
        })
    }
}
