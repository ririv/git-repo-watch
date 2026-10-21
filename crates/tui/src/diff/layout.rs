//! Visual rows carry source anchors so resizing does not reuse unrelated scroll offsets.
use super::*;
mod split;
mod unified;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Anchor {
    pub row: usize,
    pub offset: usize,
}
#[derive(Debug, Clone, Copy)]
pub(crate) enum Anchors {
    One(Anchor),
    Pair(Anchor, Anchor),
}
impl Anchors {
    pub fn preferred(self) -> Anchor {
        match self {
            Self::One(anchor) | Self::Pair(anchor, _) => anchor,
        }
    }
    fn matching(self, target: Anchor) -> Option<usize> {
        let accepts = |anchor: Anchor| {
            (anchor.row == target.row && anchor.offset <= target.offset).then_some(anchor.offset)
        };
        match self {
            Self::One(anchor) => accepts(anchor),
            Self::Pair(old, new) => accepts(old).max(accepts(new)),
        }
    }
}
#[derive(Debug)]
pub(crate) enum VisualRow {
    Full {
        line: Line<'static>,
        anchor: Anchor,
    },
    Columns {
        left: Line<'static>,
        right: Line<'static>,
        anchors: Anchors,
    },
}
impl VisualRow {
    pub fn anchors(&self) -> Anchors {
        match self {
            Self::Full { anchor, .. } => Anchors::One(*anchor),
            Self::Columns { anchors, .. } => *anchors,
        }
    }
}
#[derive(Debug)]
pub(crate) struct Cache {
    pub width: u16,
    pub mode: Layout,
    pub rows: Vec<VisualRow>,
    position: Option<Position>,
}
#[derive(Debug)]
struct Position {
    anchor: Anchor,
    scroll: usize,
}
impl Cache {
    pub fn anchor(&self, scroll: usize) -> Option<Anchor> {
        if let Some(position) = &self.position
            && position.scroll == scroll
        {
            return Some(position.anchor);
        }
        self.rows.get(scroll).map(|row| row.anchors().preferred())
    }
    pub fn settle(&mut self, scroll: usize) {
        self.position = self
            .anchor(scroll)
            .map(|anchor| Position { anchor, scroll });
    }
    pub fn restore(&mut self, anchor: Anchor, scroll: usize) {
        self.position = Some(Position { anchor, scroll });
    }
    pub fn locate(&self, target: Anchor) -> usize {
        self.rows
            .iter()
            .enumerate()
            .filter_map(|(index, row)| row.anchors().matching(target).map(|offset| (index, offset)))
            .max_by_key(|(_, offset)| *offset)
            .map(|(index, _)| index)
            .expect("every logical diff row has a visual anchor")
    }
}

pub(super) fn arrange(rows: &[Row], digits: u16, width: u16, mode: Layout) -> Cache {
    let rows = match mode {
        Layout::Unified => unified::arrange(rows, digits, width),
        Layout::Split => split::arrange(rows, digits, width),
    };
    Cache {
        width,
        mode,
        rows,
        position: None,
    }
}

fn full(out: &mut Vec<VisualRow>, row: usize, line: &Line<'_>, width: u16) {
    out.extend(
        wrap::line(line, width)
            .into_iter()
            .map(|part| VisualRow::Full {
                line: part.line,
                anchor: Anchor {
                    row,
                    offset: part.offset,
                },
            }),
    );
}
#[derive(Clone, Copy)]
enum Mark {
    Context,
    Added,
    Removed,
}
impl Mark {
    fn style(self) -> Style {
        match self {
            Self::Context => Style::default(),
            Self::Added => Style::default().bg(ADDED),
            Self::Removed => Style::default().bg(REMOVED),
        }
    }
    fn prefix(self) -> Span<'static> {
        let (text, color) = match self {
            Self::Context => (" ", MUTED),
            Self::Added => ("+", Color::LightGreen),
            Self::Removed => ("-", Color::LightRed),
        };
        Span::styled(text, self.style().fg(color))
    }
}
fn decorate(part: Line<'static>, gutter: String, mark: Mark, first: bool) -> Line<'static> {
    let base = mark.style();
    let mut spans = vec![Span::styled(gutter, base.fg(MUTED))];
    spans.push(if first {
        mark.prefix()
    } else {
        Span::styled(" ", base)
    });
    spans.extend(part.spans);
    Line::from(spans).style(base)
}
