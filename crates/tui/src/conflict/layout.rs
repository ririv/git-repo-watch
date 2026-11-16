//! The same source cells serve overview and pair-only modes inside the conflict workspace.
use super::*;
use engine::local::files::conflict::Cell;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Arrangement {
    Columns,
    Rows,
}
#[derive(Debug)]
pub(crate) struct Row {
    pub cells: [links::Line; 4],
    pub kind: diff::RowKind,
}
#[derive(Debug)]
pub(crate) struct Cache {
    pub width: u16,
    pub wrap: bool,
    pub arrangement: Arrangement,
    pub mode: DisplayMode,
    pub visible: usize,
    pub rows: Vec<Row>,
    pub anchors: Vec<usize>,
    pub maxima: [usize; 4],
}
impl View {
    pub fn arrange(
        &mut self,
        width: u16,
        wrap: bool,
        arrangement: Arrangement,
        visible: usize,
        scroll: &mut usize,
    ) {
        if self.cache.as_ref().is_some_and(|c| {
            c.width == width
                && c.wrap == wrap
                && c.arrangement == arrangement
                && c.mode == self.mode
        }) {
            self.cache.as_mut().expect("cached layout").visible = visible;
            return;
        }
        let context = visible.saturating_sub(1).min(3);
        let anchor = self
            .cache
            .as_ref()
            .and_then(|c| c.anchors.get(*scroll))
            .copied();
        let revealed = match (&self.cache, self.active_comparison()) {
            (Some(cache), Comparison::Text(rows))
                if cache.mode == DisplayMode::Overview && self.mode == DisplayMode::IncludeBase =>
            {
                cache
                    .anchors
                    .iter()
                    .skip(*scroll)
                    .take(cache.visible)
                    .copied()
                    .find(|index| {
                        rows.get(*index).is_some_and(|row| {
                            [row.base, row.current, row.incoming]
                                .iter()
                                .any(|cell| matches!(cell, Cell::Changed(_)))
                        })
                    })
                    .map(|n| n.saturating_sub(context))
            }
            _ => None,
        };
        let widths = widths(width, arrangement, self.mode);
        let mut rows = Vec::new();
        let mut anchors = Vec::new();
        let mut maxima = [0; 4];
        let count = match self.active_comparison() {
            Comparison::Text(rows) => rows.len(),
            _ => self
                .mode
                .indices()
                .iter()
                .map(|n| match self.body(*n) {
                    Some(Body::Text { lines, .. }) => lines.len(),
                    _ => 1,
                })
                .max()
                .unwrap_or(0),
        };
        for at in 0..count.max(1) {
            let parts: [Vec<links::Line>; 4] = std::array::from_fn(|n| {
                if !self.mode.indices().contains(&n) {
                    return vec![links::Line::default()];
                }
                let cell = self.cell(n, at);
                maxima[n] = maxima[n].max(if wrap {
                    0
                } else {
                    cell.width().saturating_sub(widths[n] as usize)
                });
                if wrap {
                    cell.wrap_pinned(widths[n] as usize, self.gutter(n))
                } else {
                    vec![cell]
                }
            });
            for offset in 0..parts.iter().map(Vec::len).max().unwrap_or(1) {
                rows.push(Row {
                    cells: std::array::from_fn(|n| {
                        parts[n].get(offset).cloned().unwrap_or_default()
                    }),
                    kind: diff::RowKind::Content,
                });
                anchors.push(at);
            }
            let endings: [Option<links::Line>; 4] = std::array::from_fn(|n| {
                if self.mode.indices().contains(&n) {
                    self.ending(n, at)
                } else {
                    None
                }
            });
            if endings.iter().any(Option::is_some) {
                rows.push(Row {
                    cells: endings.map(Option::unwrap_or_default),
                    kind: diff::RowKind::Annotation,
                });
                anchors.push(at);
            }
        }
        self.horizontal
            .constrain(maxima.into_iter().max().unwrap_or(0));
        let target = match (revealed.or(anchor), self.reveal) {
            (_, Reveal::At(line)) => Some(line),
            (Some(anchor), _) => Some(anchor),
            (None, Reveal::FirstChange) if *scroll == 0 => match self.active_comparison() {
                Comparison::Text(rows) => rows
                    .iter()
                    .position(|r| {
                        [r.base, r.current, r.incoming]
                            .iter()
                            .any(|cell| matches!(cell, Cell::Changed(_)))
                    })
                    .map(|n| n.saturating_sub(context)),
                _ => None,
            },
            _ => None,
        };
        if let Some(target) = target {
            *scroll = anchors.iter().position(|n| *n == target).unwrap_or(0);
        }
        self.reveal = Reveal::Retain;
        self.cache = Some(Cache {
            width,
            wrap,
            arrangement,
            mode: self.mode,
            visible,
            rows,
            anchors,
            maxima,
        });
    }
    pub fn gutter(&self, n: usize) -> usize {
        match self.body(n) {
            Some(Body::Text { gutter, .. }) => *gutter,
            _ => 0,
        }
    }
    fn ending(&self, n: usize, row: usize) -> Option<links::Line> {
        let Some(Body::Text {
            lines,
            gutter,
            source,
        }) = self.body(n)
        else {
            return None;
        };
        if source.ends_with('\n') || lines.is_empty() {
            return None;
        }
        (self.coordinate(n, row).line() == lines.len().checked_sub(1))
            .then(|| diff::notice::ending(*gutter))
    }
    fn cell(&self, n: usize, row: usize) -> links::Line {
        match self.body(n) {
            Some(Body::Notice(text)) => {
                if row == 0 {
                    links::Line::from(Line::styled(
                        text.clone(),
                        Style::default().fg(Color::DarkGray),
                    ))
                } else {
                    Default::default()
                }
            }
            None => {
                if row == 0 {
                    links::Line::from(Line::styled(
                        i18n::tr!(ConflictNonText),
                        Style::default().fg(Color::DarkGray),
                    ))
                } else {
                    Default::default()
                }
            }
            Some(Body::Text { lines, gutter, .. }) => {
                if lines.is_empty() && row == 0 {
                    return links::Line::from(Line::styled(
                        format!("{}{}", " ".repeat(*gutter), i18n::tr!(ConflictEmpty)),
                        Style::default().fg(Color::DarkGray),
                    ));
                }
                match self.coordinate(n, row) {
                    Cell::Empty => Default::default(),
                    Cell::Context(index) => lines[index].clone(),
                    Cell::Changed(index) => {
                        let mut line = lines[index].clone();
                        let removed = match self.mode {
                            DisplayMode::Pair(pair) => n == pair.indices()[0],
                            _ => n == 0,
                        };
                        line.text.style =
                            line.text
                                .style
                                .bg(if removed { diff::REMOVED } else { diff::ADDED });
                        line
                    }
                }
            }
        }
    }
}
pub(crate) fn widths(width: u16, arrangement: Arrangement, mode: DisplayMode) -> [u16; 4] {
    let mut result = [0; 4];
    let half = width.saturating_sub(1) / 2;
    if let DisplayMode::Pair(pair) = mode {
        let [left, right] = *pair.indices();
        result[left] = half;
        result[right] = width.saturating_sub(1) - half;
        return result;
    }
    if mode == DisplayMode::IncludeBase {
        result[0] = width;
    }
    result[1] = if arrangement == Arrangement::Rows {
        width
    } else {
        half
    };
    result[2] = if arrangement == Arrangement::Rows {
        width
    } else {
        width.saturating_sub(1) - half
    };
    result
}
