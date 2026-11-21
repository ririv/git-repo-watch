//! Block selection and one guarded undo survive the normal file refresh.
use engine::local::files::conflict::resolve::{self, Block, Choice, Plan};
use std::sync::Arc;

#[derive(Debug, Clone, Copy)]
pub(crate) enum Action {
    Accept(Choice),
    Undo,
}
#[derive(Debug, Clone)]
enum State {
    Unavailable(String),
    Resolved(Arc<resolve::Snapshot>),
    Blocks {
        snapshot: Arc<resolve::Snapshot>,
        selected: usize,
    },
}
#[derive(Debug, Clone, Copy)]
pub(crate) enum Reveal {
    Retain,
    Selected(usize),
}
#[derive(Debug, Clone)]
struct Undo {
    plan: Box<Plan>,
    selected: usize,
}
#[derive(Debug)]
pub(crate) struct Bookmark {
    selected: usize,
    undo: Option<Undo>,
    reveal: Reveal,
}
#[derive(Debug, Clone)]
pub(crate) struct Session {
    state: State,
    undo: Option<Undo>,
    reveal: Reveal,
}
impl Session {
    pub fn new(snapshot: anyhow::Result<resolve::Snapshot>) -> Self {
        let state = match snapshot {
            Ok(snapshot) if snapshot.blocks().is_empty() => State::Resolved(Arc::new(snapshot)),
            Ok(snapshot) => State::Blocks {
                snapshot: Arc::new(snapshot),
                selected: 0,
            },
            Err(error) => State::Unavailable(format!("{error:#}")),
        };
        Self {
            state,
            undo: None,
            reveal: Reveal::Retain,
        }
    }
    pub fn reason(&self) -> Option<&str> {
        match &self.state {
            State::Unavailable(reason) => Some(reason),
            State::Resolved(_) => Some(i18n::tr!(ConflictBlockRequired)),
            State::Blocks { .. } => None,
        }
    }
    pub fn label(&self) -> String {
        match &self.state {
            State::Blocks { snapshot, selected } => i18n::tf!(
                ConflictBlockCount,
                current = selected + 1,
                total = snapshot.blocks().len()
            ),
            State::Resolved(_) => i18n::tr!(ConflictResolvedPendingStage).into(),
            State::Unavailable(_) => String::new(),
        }
    }
    pub fn block(&self) -> Option<&Block> {
        match &self.state {
            State::Blocks { snapshot, selected } => snapshot.blocks().get(*selected),
            _ => None,
        }
    }
    pub fn prepare(&self, choice: Choice) -> anyhow::Result<Plan> {
        match &self.state {
            State::Blocks { snapshot, selected } => snapshot.prepare(*selected, choice),
            _ => anyhow::bail!("{}", self.reason().expect("unavailable resolution")),
        }
    }
    pub fn reset(&self, cancel: &engine::cancel::Cancel) -> anyhow::Result<Plan> {
        match &self.state {
            State::Blocks { snapshot, .. } | State::Resolved(snapshot) => snapshot.reset(cancel),
            State::Unavailable(reason) => anyhow::bail!("{reason}"),
        }
    }
    pub fn reset_reason(&self) -> Option<&str> {
        match &self.state {
            State::Unavailable(reason) => Some(reason),
            _ => None,
        }
    }
    pub fn resolved(&self) -> bool {
        matches!(self.state, State::Resolved(_))
    }
    pub fn move_cursor(&mut self, delta: isize) {
        if let State::Blocks { snapshot, selected } = &mut self.state {
            *selected = selected
                .saturating_add_signed(delta)
                .min(snapshot.blocks().len() - 1);
        }
    }
    pub fn block_index(&self, line: usize) -> Option<usize> {
        match &self.state {
            State::Blocks { snapshot, .. } => snapshot
                .blocks()
                .iter()
                .position(|block| block.lines.contains(&line)),
            _ => None,
        }
    }
    pub fn choose(&mut self, index: usize) -> bool {
        if let State::Blocks { snapshot, selected } = &mut self.state
            && index < snapshot.blocks().len()
        {
            *selected = index;
            return true;
        }
        false
    }
    pub fn undo(&self) -> Option<&Plan> {
        self.undo.as_ref().map(|undo| undo.plan.as_ref())
    }
    pub fn accepting(&mut self, plan: &Plan) {
        let selected = if matches!(plan.intent(), resolve::Intent::Reset) {
            0
        } else {
            match self.state {
                State::Blocks { selected, .. } => selected,
                _ => 0,
            }
        };
        self.undo = Some(Undo {
            plan: Box::new(plan.inverse()),
            selected,
        });
        self.reveal = Reveal::Selected(selected);
    }
    pub fn undoing(&mut self) {
        let undo = self.undo.take().expect("available undo action");
        self.reveal = Reveal::Selected(undo.selected);
    }
    pub fn bookmark(&self) -> Bookmark {
        Bookmark {
            selected: match self.state {
                State::Blocks { selected, .. } => selected,
                _ => 0,
            },
            undo: self.undo.clone(),
            reveal: self.reveal,
        }
    }
    pub fn restore(&mut self, bookmark: Bookmark, source: Option<&str>) -> Reveal {
        if let State::Blocks { snapshot, selected } = &mut self.state {
            *selected = match bookmark.reveal {
                Reveal::Selected(index) => index,
                Reveal::Retain => bookmark.selected,
            } % snapshot.blocks().len();
        }
        self.undo = bookmark
            .undo
            .filter(|undo| source.is_some_and(|source| undo.plan.expects(source)));
        bookmark.reveal
    }
}
