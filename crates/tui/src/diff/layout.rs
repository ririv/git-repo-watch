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
    pub(crate) fn contains(self, target: Anchor) -> bool {
        match self {
            Self::One(anchor) => anchor == target,
            Self::Pair(old, new) => old == target || new == target,
        }
    }
}
#[derive(Debug)]
pub(crate) enum VisualRow {
    Full {
        line: crate::links::Line,
        fixed: usize,
        anchor: Anchor,
        kind: RowKind,
    },
    Columns {
        left: crate::links::Line,
        right: crate::links::Line,
        anchors: Anchors,
        fixed: usize,
        kind: RowKind,
    },
}
/// Annotations share scroll anchors with source rows but never select source lines.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RowKind {
    Content,
    Annotation,
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
    pub wrap: bool,
    pub context: Context,
    pub ids: Vec<Option<engine::local::files::partial::LineId>>,
    pub(super) coordinates: Vec<super::position::Coordinates>,
    paths: Vec<std::path::PathBuf>,
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
    pub fn matches_paths(&self, paths: &[std::path::PathBuf]) -> bool {
        self.paths == paths
    }
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

pub(super) fn arrange(
    rows: &[Row],
    digits: u16,
    width: u16,
    mode: Layout,
    wrap: bool,
    paths: &[std::path::PathBuf],
) -> Cache {
    let ids = rows.iter().map(Row::line_id).collect();
    let coordinates = rows.iter().map(Row::coordinates).collect();
    let rows = match mode {
        Layout::Unified => unified::arrange(rows, digits),
        Layout::Split => split::arrange(rows, digits),
    };
    let mut rows = rows;
    for row in &mut rows {
        match row {
            VisualRow::Full { line, .. } => line.detect(paths),
            VisualRow::Columns { left, right, .. } => {
                left.detect(paths);
                right.detect(paths);
            }
        }
    }
    let rows = if wrap { wrapped(rows, width) } else { rows };
    let maximum = if wrap {
        0
    } else {
        horizontal_maximum(&rows, width)
    };
    Cache {
        width,
        mode,
        wrap,
        context: Context::Changes,
        ids,
        coordinates,
        paths: paths.to_vec(),
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
        line: line.clone().into(),
        fixed: 0,
        anchor: Anchor { row },
        kind: RowKind::Content,
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
fn decorate(part: Line<'static>, gutter: String, mark: Mark) -> crate::links::Line {
    let base = mark.style();
    let mut spans = vec![Span::styled(gutter, base.fg(MUTED))];
    spans.push(mark.prefix());
    spans.push(Span::styled("│", base.fg(MUTED)));
    spans.extend(part.spans);
    Line::from(spans).style(base).into()
}

fn wrapped(rows: Vec<VisualRow>, width: u16) -> Vec<VisualRow> {
    let width = usize::from(width);
    let left_width = width.saturating_sub(1) / 2;
    let right_width = width.saturating_sub(1) - left_width;
    let wrap = |line: crate::links::Line, width, fixed| line.wrap_pinned(width, fixed);
    rows.into_iter()
        .flat_map(|row| match row {
            VisualRow::Full {
                line,
                fixed,
                anchor,
                kind,
            } => wrap(line, width, fixed)
                .into_iter()
                .map(|line| VisualRow::Full {
                    line,
                    fixed,
                    anchor,
                    kind,
                })
                .collect(),
            VisualRow::Columns {
                left,
                right,
                anchors,
                fixed,
                kind,
            } => {
                let left_style = left.text.style;
                let right_style = right.text.style;
                let left = wrap(left, left_width, fixed);
                let right = wrap(right, right_width, fixed);
                (0..left.len().max(right.len()))
                    .map(|index| VisualRow::Columns {
                        left: left
                            .get(index)
                            .cloned()
                            .unwrap_or_else(|| Line::default().style(left_style).into()),
                        right: right
                            .get(index)
                            .cloned()
                            .unwrap_or_else(|| Line::default().style(right_style).into()),
                        anchors,
                        fixed,
                        kind,
                    })
                    .collect::<Vec<_>>()
            }
        })
        .collect()
}
