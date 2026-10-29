//! A non-empty commit selection owns the actions offered by its modal menu.
use engine::local::history::Oid;
use ratatui::widgets::ListState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Operation {
    CherryPick,
    Revert(Oid),
    Copy,
    Clear,
    Done,
}
impl Operation {
    pub fn label(self) -> &'static str {
        match self {
            Self::CherryPick => i18n::tr!(HistoryActionCherryPick),
            Self::Revert(_) => i18n::tr!(HistoryActionRevert),
            Self::Copy => i18n::tr!(HistoryActionCopy),
            Self::Clear => i18n::tr!(HistorySelectClear),
            Self::Done => i18n::tr!(HistorySelectDone),
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PointerInput {
    Choose(usize),
    Close,
    Stay,
}

#[derive(Debug, Clone)]
pub(crate) struct Menu {
    first: Oid,
    rest: Vec<Oid>,
    operations: Vec<Operation>,
    pub state: ListState,
}
impl Menu {
    pub fn new(commits: Vec<Oid>, replay_allowed: bool) -> Option<Self> {
        let mut commits = commits.into_iter();
        let first = commits.next()?;
        let rest = commits.collect::<Vec<_>>();
        let mut operations = Vec::new();
        if replay_allowed {
            operations.push(Operation::CherryPick);
            if rest.is_empty() {
                operations.push(Operation::Revert(first));
            }
        }
        operations.extend([Operation::Copy, Operation::Clear, Operation::Done]);
        Some(Self {
            first,
            rest,
            operations,
            state: ListState::default().with_selected(Some(0)),
        })
    }
    pub fn commits(&self) -> Vec<Oid> {
        std::iter::once(self.first)
            .chain(self.rest.iter().copied())
            .collect()
    }
    pub fn contains(&self, oid: &Oid) -> bool {
        self.first == *oid || self.rest.contains(oid)
    }
    pub fn count(&self) -> usize {
        self.rest.len() + 1
    }
    pub fn operations(&self) -> &[Operation] {
        &self.operations
    }
    pub fn choose(&self, index: usize) -> Option<Operation> {
        self.operations.get(index).copied()
    }
    pub fn advance(&mut self, delta: isize) {
        self.seek(
            self.state
                .selected()
                .unwrap_or(0)
                .saturating_add_signed(delta),
        );
    }
    pub fn seek(&mut self, index: usize) {
        self.state
            .select(Some(index.min(self.operations.len() - 1)));
    }
}
