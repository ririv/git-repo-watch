//! Visible matches always form the same forest, independently of command-group navigation.
use super::{Command, Group, Input, descendant};

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
    candidates: &'a [&'static Command],
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
impl<'a> Tree<'a> {
    pub fn new(group: Option<&Group>, candidates: &'a [&'static Command]) -> Self {
        Self {
            root: match group {
                Some(group) => Root::Group(group.parent),
                None => Root::Search,
            },
            candidates,
        }
    }
    fn parent_of(&self, command: &Command) -> Option<&'static Command> {
        self.candidates
            .iter()
            .copied()
            .filter(|ancestor| descendant(command.name, ancestor.name))
            .max_by_key(|ancestor| ancestor.name.len())
            .or_else(|| match self.root {
                Root::Group(parent) if descendant(command.name, parent.name) => Some(parent),
                Root::Search | Root::Group(_) => None,
            })
    }
    fn has_next_sibling(&self, command: &Command) -> bool {
        let index = self
            .candidates
            .iter()
            .position(|candidate| candidate.name == command.name)
            .expect("visible command");
        let parent = self.parent_of(command).map(|parent| parent.name);
        self.candidates[index + 1..]
            .iter()
            .skip_while(|next| descendant(next.name, command.name))
            .next()
            .is_some_and(|next| self.parent_of(next).map(|parent| parent.name) == parent)
    }
    pub fn label(&self, input: &Input, command: &Command) -> Label {
        let Some(parent) = self.parent_of(command) else {
            let prefix = if input.menu_context().is_some() && !command.common() {
                ""
            } else {
                "/"
            };
            return Label {
                branch: String::new(),
                name: format!("{prefix}{}", input.name(command)),
            };
        };
        let mut ancestors = self
            .candidates
            .iter()
            .copied()
            .filter(|ancestor| {
                descendant(command.name, ancestor.name) && self.parent_of(ancestor).is_some()
            })
            .collect::<Vec<_>>();
        ancestors.sort_by_key(|ancestor| ancestor.name.len());
        let mut branch = String::new();
        for ancestor in ancestors {
            branch.push_str(if self.has_next_sibling(ancestor) {
                "│  "
            } else {
                "   "
            });
        }
        branch.push_str(if self.has_next_sibling(command) {
            "├─ "
        } else {
            "└─ "
        });
        Label {
            branch,
            name: command
                .name
                .strip_prefix(parent.name)
                .expect("command descendant")
                .trim_start()
                .to_owned(),
        }
    }
}
