//! A single hunk or contiguous changed-line range belongs to an immutable patch snapshot.
use engine::local::files::partial::{LineId, Snapshot};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Granularity {
    Hunk,
    Line,
}
impl Granularity {
    pub fn label(self) -> &'static str {
        match self {
            Self::Hunk => i18n::tr!(PartialHunk),
            Self::Line => i18n::tr!(PartialLine),
        }
    }
}
#[derive(Debug)]
pub(super) struct Bookmark {
    path: std::path::PathBuf,
    mode: Granularity,
    cursor: LineId,
}
#[derive(Debug)]
enum Selection {
    Hunk(usize),
    Lines { anchor: usize, cursor: usize },
}
#[derive(Debug)]
pub(crate) enum State {
    Unavailable(String),
    Ready(Box<Session>),
}
#[derive(Debug)]
pub(crate) struct Session {
    pub snapshot: Snapshot,
    lines: Vec<LineId>,
    selection: Selection,
}
impl State {
    pub fn new(snapshot: anyhow::Result<Snapshot>) -> Self {
        match snapshot {
            Ok(snapshot) => Self::Ready(Box::new(Session {
                lines: snapshot.hunks().iter().flatten().copied().collect(),
                snapshot,
                selection: Selection::Hunk(0),
            })),
            Err(error) => Self::Unavailable(format!("{error:#}")),
        }
    }
}
impl Session {
    pub(super) fn bookmark(&self) -> Bookmark {
        Bookmark {
            path: self.snapshot.change().location.name().to_owned(),
            mode: self.mode(),
            cursor: self.cursor(),
        }
    }
    pub(super) fn restore(&mut self, bookmark: &Bookmark) {
        if self.snapshot.change().location.name() != bookmark.path {
            return;
        }
        self.set_mode(bookmark.mode);
        let coordinate = |id: LineId| match id {
            LineId::Old(number) => (0, number.get()),
            LineId::New(number) => (1, number.get()),
        };
        let (side, line) = coordinate(bookmark.cursor);
        let closest = self
            .lines
            .iter()
            .min_by_key(|&&id| {
                let (next_side, next_line) = coordinate(id);
                (next_line.abs_diff(line), next_side != side)
            })
            .copied()
            .expect("editable patch has changed lines");
        self.choose(closest);
    }
    pub fn label(&self) -> String {
        match self.selection {
            Selection::Hunk(index) => format!(
                "{} {}/{}",
                self.mode().label(),
                index + 1,
                self.snapshot.hunks().len()
            ),
            Selection::Lines { anchor, cursor } => format!(
                "{} {}–{}/{}",
                self.mode().label(),
                anchor.min(cursor) + 1,
                anchor.max(cursor) + 1,
                self.lines.len()
            ),
        }
    }
    pub fn mode(&self) -> Granularity {
        match self.selection {
            Selection::Hunk(_) => Granularity::Hunk,
            Selection::Lines { .. } => Granularity::Line,
        }
    }
    pub fn cursor(&self) -> LineId {
        match self.selection {
            Selection::Hunk(index) => self.snapshot.hunks()[index][0],
            Selection::Lines { cursor, .. } => self.lines[cursor],
        }
    }
    pub fn selected(&self) -> BTreeSet<LineId> {
        match self.selection {
            Selection::Hunk(index) => self.snapshot.hunks()[index].iter().copied().collect(),
            Selection::Lines { anchor, cursor } => self.lines
                [anchor.min(cursor)..=anchor.max(cursor)]
                .iter()
                .copied()
                .collect(),
        }
    }
    pub fn set_mode(&mut self, mode: Granularity) {
        if self.mode() == mode {
            return;
        }
        let cursor = self.cursor();
        self.selection = match mode {
            Granularity::Hunk => Selection::Hunk(
                self.snapshot
                    .hunks()
                    .iter()
                    .position(|hunk| hunk.contains(&cursor))
                    .expect("cursor belongs to a hunk"),
            ),
            Granularity::Line => {
                let cursor = self
                    .lines
                    .iter()
                    .position(|id| *id == cursor)
                    .expect("cursor belongs to patch");
                Selection::Lines {
                    anchor: cursor,
                    cursor,
                }
            }
        };
    }
    pub fn advance(&mut self, delta: isize, extend: bool) {
        if extend && self.mode() == Granularity::Hunk {
            self.set_mode(Granularity::Line);
        }
        match &mut self.selection {
            Selection::Hunk(index) => {
                *index = index
                    .saturating_add_signed(delta)
                    .min(self.snapshot.hunks().len() - 1)
            }
            Selection::Lines { anchor, cursor } => {
                *cursor = cursor
                    .saturating_add_signed(delta)
                    .min(self.lines.len() - 1);
                if !extend {
                    *anchor = *cursor;
                }
            }
        }
    }
    pub fn choose(&mut self, id: LineId) {
        let Some(cursor) = self.lines.iter().position(|line| *line == id) else {
            return;
        };
        self.selection = match self.selection {
            Selection::Hunk(_) => Selection::Hunk(
                self.snapshot
                    .hunks()
                    .iter()
                    .position(|hunk| hunk.contains(&id))
                    .expect("line belongs to hunk"),
            ),
            Selection::Lines { .. } => Selection::Lines {
                anchor: cursor,
                cursor,
            },
        };
    }
}
