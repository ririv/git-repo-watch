//! Visual rows carry source anchors so resizing does not reuse unrelated scroll offsets.
use super::*;
mod split;
mod unified;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Anchor {
    pub row: usize,
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
    fn contains(self, target: Anchor) -> bool {
        match self {
            Self::One(anchor) => anchor == target,
            Self::Pair(old, new) => old == target || new == target,
        }
    }
}
#[derive(Debug)]
pub(crate) enum VisualRow {
    Full {
        line: Line<'static>,
        fixed: usize,
        anchor: Anchor,
    },
    Columns {
        left: Line<'static>,
        right: Line<'static>,
        anchors: Anchors,
        fixed: usize,
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
    maximum: usize,
    position: Option<Position>,
}
#[derive(Debug)]
struct Position {
    anchor: Anchor,
    scroll: usize,
}
impl Cache {
    pub fn horizontal_maximum(&self) -> usize {
        self.maximum
    }
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
            .position(|row| row.anchors().contains(target))
            .expect("every logical diff row has a visual anchor")
    }
}

pub(super) fn arrange(rows: &[Row], digits: u16, width: u16, mode: Layout) -> Cache {
    let rows = match mode {
        Layout::Unified => unified::arrange(rows, digits),
        Layout::Split => split::arrange(rows, digits),
    };
    let maximum = horizontal_maximum(&rows, width);
    Cache {
        width,
        mode,
        rows,
        maximum,
        position: None,
    }
}

fn horizontal_maximum(rows: &[VisualRow], width: u16) -> usize {
    let width = usize::from(width);
    let left_width = width.saturating_sub(1) / 2;
    let right_width = width.saturating_sub(1) - left_width;
    rows.iter()
        .map(|row| match row {
            VisualRow::Full { line, .. } => line.width().saturating_sub(width),
            VisualRow::Columns { left, right, .. } => left
                .width()
                .saturating_sub(left_width)
                .max(right.width().saturating_sub(right_width)),
        })
        .max()
        .unwrap_or(0)
}

fn full(out: &mut Vec<VisualRow>, row: usize, line: &Line<'static>) {
    out.push(VisualRow::Full {
        line: line.clone(),
        fixed: 0,
        anchor: Anchor { row },
    });
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
fn decorate(part: Line<'static>, gutter: String, mark: Mark) -> Line<'static> {
    let base = mark.style();
    let mut spans = vec![Span::styled(gutter, base.fg(MUTED))];
    spans.push(mark.prefix());
    spans.extend(part.spans);
    Line::from(spans).style(base)
}
