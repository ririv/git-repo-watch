//! Commit marks exist only inside an explicitly entered selection mode.
use engine::local::history::Oid;
use std::collections::BTreeSet;

#[derive(Debug, Clone, Default)]
pub(crate) enum Selection {
    #[default]
    Browsing,
    Selecting(BTreeSet<Oid>),
}
impl Selection {
    pub fn active(&self) -> bool {
        matches!(self, Self::Selecting(_))
    }
    pub fn count(&self) -> usize {
        match self {
            Self::Browsing => 0,
            Self::Selecting(marked) => marked.len(),
        }
    }
    pub fn contains(&self, oid: &Oid) -> bool {
        matches!(self, Self::Selecting(marked) if marked.contains(oid))
    }
    pub fn begin(&mut self) -> &mut BTreeSet<Oid> {
        if matches!(self, Self::Browsing) {
            *self = Self::Selecting(BTreeSet::new());
        }
        match self {
            Self::Selecting(marked) => marked,
            Self::Browsing => unreachable!(),
        }
    }
    pub fn finish(&mut self) {
        *self = Self::Browsing;
    }
}
