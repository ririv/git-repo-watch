//! Commit marks exist only inside an explicitly entered selection mode.
use anyhow::{Result, ensure};
use engine::local::history::Oid;
use std::collections::BTreeSet;

#[derive(Debug, Clone, Default)]
pub(crate) enum Selection {
    #[default]
    Browsing,
    Selecting(BTreeSet<Oid>),
    Actions(Box<super::menu::Menu>),
    Range {
        anchor: usize,
        tip: usize,
        base: BTreeSet<Oid>,
        range: BTreeSet<Oid>,
    },
}

impl Selection {
    pub fn active(&self) -> bool {
        !matches!(self, Self::Browsing)
    }
    pub fn count(&self) -> usize {
        match self {
            Self::Browsing => 0,
            Self::Selecting(marked) => marked.len(),
            Self::Actions(menu) => menu.count(),
            Self::Range { base, range, .. } => base.union(range).count(),
        }
    }
    pub fn contains(&self, oid: &Oid) -> bool {
        match self {
            Self::Browsing => false,
            Self::Selecting(marked) => marked.contains(oid),
            Self::Actions(menu) => menu.contains(oid),
            Self::Range { base, range, .. } => base.contains(oid) || range.contains(oid),
        }
    }
    pub fn begin(&mut self) -> &mut BTreeSet<Oid> {
        *self = match std::mem::take(self) {
            Self::Browsing => Self::Selecting(BTreeSet::new()),
            Self::Actions(menu) => Self::Selecting(menu.commits().into_iter().collect()),
            Self::Range {
                mut base, range, ..
            } => {
                base.extend(range);
                Self::Selecting(base)
            }
            selecting => selecting,
        };
        match self {
            Self::Selecting(marked) => marked,
            Self::Browsing | Self::Range { .. } | Self::Actions(_) => unreachable!(),
        }
    }
    pub fn settle(&mut self) {
        if matches!(self, Self::Range { .. }) {
            self.begin();
        }
    }
    pub fn extend(&mut self, from: usize, tip: usize, oid: impl Fn(usize) -> Oid) -> Result<()> {
        let (anchor, base) = match self {
            Self::Browsing | Self::Actions(_) => return Ok(()),
            Self::Selecting(marked) => (from, marked.clone()),
            Self::Range {
                anchor, tip, base, ..
            } if *tip == from => (*anchor, base.clone()),
            Self::Range { base, range, .. } => (from, base.union(range).copied().collect()),
        };
        ensure!(
            anchor.abs_diff(tip) < engine::operations::replay::batch::MAX_COMMITS,
            i18n::tr!(ReplayBatchTooLarge)
        );
        let range = (anchor.min(tip)..=anchor.max(tip))
            .map(oid)
            .collect::<BTreeSet<_>>();
        ensure!(
            base.union(&range).count() <= engine::operations::replay::batch::MAX_COMMITS,
            i18n::tr!(ReplayBatchTooLarge)
        );
        *self = Self::Range {
            anchor,
            tip,
            base,
            range,
        };
        Ok(())
    }
    pub fn finish(&mut self) {
        *self = Self::Browsing;
    }
    pub fn open_actions(&mut self, commits: Vec<Oid>, replay_allowed: bool) -> bool {
        let Some(menu) = super::menu::Menu::new(commits, replay_allowed) else {
            return false;
        };
        *self = Self::Actions(Box::new(menu));
        true
    }
    pub fn menu(&self) -> Option<&super::menu::Menu> {
        match self {
            Self::Actions(menu) => Some(menu),
            _ => None,
        }
    }
    pub fn menu_mut(&mut self) -> Option<&mut super::menu::Menu> {
        match self {
            Self::Actions(menu) => Some(menu),
            _ => None,
        }
    }
    pub fn close_actions(&mut self) {
        if matches!(self, Self::Actions(_)) {
            self.begin();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn oid(index: usize) -> Oid {
        let mut bytes = [0; 20];
        bytes[12..].copy_from_slice(&(index as u64).to_be_bytes());
        Oid::from_bytes(&bytes).unwrap()
    }
    fn marked(selection: &Selection) -> Vec<usize> {
        (0..520)
            .filter(|index| selection.contains(&oid(*index)))
            .collect()
    }
    #[test]
    fn reversing_a_range_preserves_individual_marks_on_both_sides_of_the_anchor() {
        let mut selection = Selection::default();
        selection.begin().extend([oid(2), oid(5)]);
        selection.extend(3, 6, oid).unwrap();
        assert_eq!(marked(&selection), [2, 3, 4, 5, 6]);
        selection.extend(6, 4, oid).unwrap();
        assert_eq!(marked(&selection), [2, 3, 4, 5]);
        selection.extend(4, 1, oid).unwrap();
        assert_eq!(marked(&selection), [1, 2, 3, 5]);
        selection.extend(1, 3, oid).unwrap();
        assert_eq!(marked(&selection), [2, 3, 5]);
    }
    #[test]
    fn settling_a_range_preserves_it_when_starting_a_separate_range() {
        let mut selection = Selection::default();
        selection.begin().insert(oid(0));
        selection.extend(2, 4, oid).unwrap();
        selection.settle();
        selection.extend(6, 7, oid).unwrap();
        assert_eq!(marked(&selection), [0, 2, 3, 4, 6, 7]);
        selection.extend(7, 6, oid).unwrap();
        assert_eq!(marked(&selection), [0, 2, 3, 4, 6]);
    }
    #[test]
    fn reaching_the_selection_limit_retains_the_last_successful_range() {
        let mut selection = Selection::default();
        selection.begin().insert(oid(513));
        selection.extend(0, 510, oid).unwrap();
        assert_eq!(selection.count(), 512);
        assert!(selection.extend(510, 511, oid).is_err());
        assert_eq!(selection.count(), 512);
        assert!(!selection.contains(&oid(511)));
        selection.extend(510, 509, oid).unwrap();
        assert_eq!(selection.count(), 511);
        assert!(selection.contains(&oid(513)));
    }
}
