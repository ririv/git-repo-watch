//! Visible matches always form the same forest, independently of command-group navigation.
use super::{Command, Group, Input, descendant};
use crate::command::named::Candidate;

pub(crate) struct Label {
    pub branch: String,
    pub name: String,
}
enum Root {
    Search,
    Group(&'static Command),
}
pub(crate) struct Tree<'a> {
    root: Root,
    candidates: &'a [Candidate],
}
pub(crate) fn arrange(candidates: &mut [&'static Command]) {
    let original = candidates.to_vec();
    // 子树以其中最靠前的匹配排序；父项与子项始终相邻，跨组搜索不丢失命中。
    candidates.sort_by_cached_key(|command| {
        let mut chain = original
            .iter()
            .filter(|ancestor| {
                command.name == ancestor.name || descendant(command.name, ancestor.name)
            })
            .collect::<Vec<_>>();
        chain.sort_by_key(|ancestor| ancestor.name.len());
        chain
            .into_iter()
            .map(|ancestor| {
                original
                    .iter()
                    .position(|matched| {
                        matched.name == ancestor.name || descendant(matched.name, ancestor.name)
                    })
                    .expect("visible subtree")
            })
            .collect::<Vec<_>>()
    });
}
fn belongs_to(candidate: Candidate, command: &Command) -> bool {
    let owner = candidate.owner().name;
    descendant(owner, command.name)
        || (matches!(candidate, Candidate::Parameter(_)) && owner == command.name)
}

impl<'a> Tree<'a> {
    pub fn new(group: Option<&Group>, candidates: &'a [Candidate]) -> Self {
        Self {
            root: match group {
                Some(group) => Root::Group(group.parent),
                None => Root::Search,
            },
            candidates,
        }
    }

    fn parent_of(&self, candidate: Candidate) -> Option<&'static Command> {
        self.candidates
            .iter()
            .filter_map(|row| row.command())
            .filter(|ancestor| belongs_to(candidate, ancestor))
            .max_by_key(|ancestor| ancestor.name.len())
            .or_else(|| match self.root {
                Root::Group(parent) if belongs_to(candidate, parent) => Some(parent),
                Root::Search | Root::Group(_) => None,
            })
    }

    fn has_next_sibling(&self, index: usize) -> bool {
        let candidate = self.candidates[index];
        let parent = self.parent_of(candidate).map(|parent| parent.name);
        self.candidates[index + 1..]
            .iter()
            .copied()
            .find(|next| {
                candidate
                    .command()
                    .is_none_or(|command| !belongs_to(*next, command))
            })
            .is_some_and(|next| self.parent_of(next).map(|parent| parent.name) == parent)
    }

    pub fn label(&self, input: &Input, index: usize) -> Label {
        let candidate = self.candidates[index];
        let parent = self.parent_of(candidate);
        let name = match (candidate, parent) {
            (Candidate::Parameter(parameter), _) => parameter.name.into(),
            (Candidate::Command(command), Some(parent)) => command
                .name
                .strip_prefix(parent.name)
                .expect("command descendant")
                .trim_start()
                .to_owned(),
            (Candidate::Command(command), None) => {
                let prefix = if input.menu_context().is_some() && !command.common() {
                    ""
                } else {
                    "/"
                };
                format!("{prefix}{}", input.name(command))
            }
        };
        let Some(mut ancestor) = parent else {
            return Label {
                branch: String::new(),
                name,
            };
        };
        let mut ancestors = Vec::new();
        while let Some(ancestor_index) = self.candidates.iter().position(|row| {
            row.command()
                .is_some_and(|command| command.name == ancestor.name)
        }) {
            let Some(ancestor_parent) = self.parent_of(self.candidates[ancestor_index]) else {
                break;
            };
            ancestors.push(ancestor_index);
            ancestor = ancestor_parent;
        }
        let mut branch = String::new();
        for ancestor_index in ancestors.into_iter().rev() {
            branch.push_str(if self.has_next_sibling(ancestor_index) {
                "│  "
            } else {
                "   "
            });
        }
        branch.push_str(if self.has_next_sibling(index) {
            "├─ "
        } else {
            "└─ "
        });
        Label { branch, name }
    }
}
