//! A selected suffix of a typed command context expands along its ancestry.
use super::Context;
use std::ops::Range;

#[derive(Debug, Clone, Copy)]
enum Scope {
    Global,
    Within(Context),
}
impl Scope {
    fn from_parent(parent: Option<Context>) -> Self {
        match parent {
            Some(parent) => Self::Within(parent),
            None => Self::Global,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Selection {
    origin: Context,
    scope: Scope,
}
impl Selection {
    pub fn last(origin: Context) -> Self {
        Self {
            origin,
            scope: Scope::from_parent(origin.parent()),
        }
    }
    pub fn origin(&self) -> Context {
        self.origin
    }
    pub fn parent(&self) -> Option<Context> {
        match self.scope {
            Scope::Global => None,
            Scope::Within(parent) => Some(parent),
        }
    }
    pub fn range(&self) -> Range<usize> {
        let start = match self.scope {
            Scope::Global => 0,
            Scope::Within(parent) => parent.name().len() + 2,
        };
        start..self.origin.name().len() + 1
    }
    pub fn expand(&mut self) {
        if let Scope::Within(parent) = self.scope {
            self.scope = Scope::from_parent(parent.parent());
        }
    }
    /// Return false when the last selected segment would be deselected.
    pub fn shrink(&mut self) -> bool {
        let mut child = self.origin;
        while child.parent() != self.parent() {
            child = child.parent().expect("selection scope is an ancestor");
        }
        if child == self.origin {
            return false;
        }
        self.scope = Scope::Within(child);
        true
    }
}
