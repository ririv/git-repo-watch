//! Three immutable Git stages and the editable worktree retain independent viewports.
pub(crate) mod layout;
pub(crate) mod pair;
pub(crate) mod result;
use crate::{diff, horizontal, links};
use engine::{
    cancel::Cancel,
    local::files::{
        conflict::{Comparison, Role, Snapshot, Version},
        patch::Source,
    },
};
use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
};
use std::{collections::BTreeSet, num::NonZeroU32};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Focus {
    Compare,
    Result,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DisplayMode {
    Overview,
    IncludeBase,
    Pair(pair::Pair),
}
impl DisplayMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Overview => i18n::tr!(ConflictOverview),
            Self::IncludeBase => i18n::tr!(ConflictIncludeBase),
            Self::Pair(pair) => pair.label(),
        }
    }
    pub fn indices(self) -> &'static [usize] {
        match self {
            Self::Overview => &[1, 2],
            Self::IncludeBase => &[0, 1, 2],
            Self::Pair(pair) => pair.indices(),
        }
    }
    pub fn comparison_slot(self) -> usize {
        match self {
            Self::Overview | Self::IncludeBase => 0,
            Self::Pair(pair) => 2 + pair.index(),
        }
    }
}
const VIEWPORTS: usize = 2 + pair::Pair::ALL.len();
#[derive(Debug)]
pub(crate) enum Body {
    Text {
        lines: Vec<links::Line>,
        gutter: usize,
        source: std::sync::Arc<str>,
    },
    Notice(String),
}
#[derive(Debug)]
pub(crate) struct Column {
    pub role: Role,
    pub description: String,
    pub body: Body,
}
#[derive(Debug, Clone, Copy)]
pub(crate) enum Reveal {
    FirstChange,
    Retain,
}
#[derive(Debug)]
pub(crate) struct View {
    pub title: String,
    pub pairs: pair::Pairs,
    pub columns: [Column; 3],
    pub comparison: Comparison,
    overview_focus: Focus,
    pub mode: DisplayMode,
    pub result: result::View,
    pub horizontal: horizontal::Viewport,
    pub cache: Option<layout::Cache>,
    scrolls: [usize; VIEWPORTS],
    reveal: Reveal,
    offsets: [usize; VIEWPORTS],
}
#[derive(Debug)]
pub(crate) struct Position {
    overview_focus: Focus,
    mode: DisplayMode,
    scrolls: [usize; VIEWPORTS],
    offsets: [usize; VIEWPORTS],
}
impl View {
    pub fn prepare(
        snapshot: Snapshot,
        result: result::View,
        pairs: pair::Pairs,
        cancel: &Cancel,
    ) -> anyhow::Result<Self> {
        let Snapshot {
            path,
            base,
            current,
            incoming,
            comparison,
        } = snapshot;
        let modes = [&base, &current, &incoming].map(|v| match v {
            Version::Absent => None,
            Version::Present { mode, .. } => Some(*mode),
        });
        let show_mode = modes
            .iter()
            .flatten()
            .any(|mode| modes.iter().flatten().any(|other| mode != other));
        Ok(Self {
            title: path.display().to_string(),
            pairs,
            columns: [
                column(Role::Base, base, show_mode, &path, cancel)?,
                column(Role::Current, current, show_mode, &path, cancel)?,
                column(Role::Incoming, incoming, show_mode, &path, cancel)?,
            ],
            comparison,
            overview_focus: Focus::Result,
            mode: DisplayMode::Overview,
            result,
            horizontal: Default::default(),
            cache: None,
            scrolls: [0; VIEWPORTS],
            offsets: [0; VIEWPORTS],
            reveal: Reveal::FirstChange,
        })
    }
    pub fn focus(&self) -> Focus {
        match self.mode {
            DisplayMode::Pair(_) => Focus::Compare,
            _ => self.overview_focus,
        }
    }
    pub fn select(&mut self, next: Focus, scroll: &mut usize) {
        if matches!(self.mode, DisplayMode::Pair(_)) {
            return;
        }
        self.scrolls[self.slot()] = *scroll;
        self.overview_focus = next;
        *scroll = self.scrolls[self.slot()];
    }
    pub fn set_mode(&mut self, next: DisplayMode, scroll: &mut usize) {
        if next == self.mode {
            return;
        }
        self.scrolls[self.slot()] = *scroll;
        self.offsets[self.mode.comparison_slot()] = self.horizontal.offset();
        let changed_pair = self.mode.comparison_slot() != next.comparison_slot();
        self.mode = next;
        *scroll = self.scrolls[self.slot()];
        let offset = self.offsets[self.mode.comparison_slot()];
        self.horizontal.constrain(offset);
        self.horizontal.seek(offset);
        if changed_pair {
            self.cache = None;
            self.reveal = if *scroll == 0 {
                Reveal::FirstChange
            } else {
                Reveal::Retain
            };
        }
    }
    fn slot(&self) -> usize {
        match self.mode {
            DisplayMode::Pair(target) => 2 + target.index(),
            _ => match self.overview_focus {
                Focus::Compare => 0,
                Focus::Result => 1,
            },
        }
    }
    pub fn position(&self, scroll: usize) -> Position {
        let mut offsets = self.offsets;
        offsets[self.mode.comparison_slot()] = self.horizontal.offset();
        offsets[1] = self.result.offset();
        Position {
            overview_focus: self.overview_focus,
            mode: self.mode,
            scrolls: self.scrolls(scroll),
            offsets,
        }
    }
    pub fn restore(&mut self, position: Position, scroll: &mut usize) {
        self.reveal = Reveal::Retain;
        self.result.retain();
        self.overview_focus = position.overview_focus;
        self.mode = position.mode;
        self.scrolls = position.scrolls;
        self.offsets = position.offsets;
        let offset = self.offsets[self.mode.comparison_slot()];
        self.horizontal.constrain(offset);
        self.horizontal.seek(offset);
        self.result.restore_offset(self.offsets[1]);
        *scroll = self.scrolls[self.slot()];
    }
    pub fn scrolls(&self, scroll: usize) -> [usize; VIEWPORTS] {
        let mut positions = self.scrolls;
        positions[self.slot()] = scroll;
        positions
    }
    pub fn settle(&mut self, positions: [usize; VIEWPORTS], scroll: &mut usize) {
        self.scrolls = positions;
        *scroll = positions[self.slot()];
    }
    pub fn active_comparison(&self) -> &Comparison {
        match self.mode {
            DisplayMode::Pair(target) => self.pairs.get(target),
            _ => &self.comparison,
        }
    }
    pub fn body(&self, n: usize) -> Option<&Body> {
        if n < 3 {
            Some(&self.columns[n].body)
        } else {
            match &self.result.content {
                result::Content::Source { body, .. } => Some(body),
                result::Content::Document(_) => None,
            }
        }
    }
    pub fn description(&self, n: usize) -> &str {
        if n < 3 {
            &self.columns[n].description
        } else {
            match &self.result.content {
                result::Content::Source { syntax, .. } => syntax,
                result::Content::Document(_) => i18n::tr!(ConflictNonText),
            }
        }
    }
    pub fn coordinate(&self, n: usize, row: usize) -> engine::local::files::conflict::Cell {
        use engine::local::files::conflict::Cell;
        match self.active_comparison() {
            Comparison::Text(rows) => rows
                .get(row)
                .map(|r| {
                    if let DisplayMode::Pair(pair) = self.mode {
                        if n == pair.indices()[0] {
                            r.base
                        } else {
                            r.current
                        }
                    } else {
                        match n {
                            0 => r.base,
                            1 => r.current,
                            _ => r.incoming,
                        }
                    }
                })
                .unwrap_or(Cell::Empty),
            _ => match self.body(n) {
                Some(Body::Text { lines, .. }) if row < lines.len() => Cell::Context(row),
                _ => Cell::Empty,
            },
        }
    }
    pub fn export(&self) -> String {
        let mut text = String::new();
        for index in self.mode.indices() {
            let (label, description) = if *index < 3 {
                let column = &self.columns[*index];
                (column.role.label(), column.description.as_str())
            } else {
                (i18n::tr!(ConflictResult), self.description(*index))
            };
            text.push_str(&format!("{}\n{}\n", label, description));
            match self.body(*index) {
                Some(Body::Notice(_)) | None => {}
                Some(Body::Text { source, .. }) => {
                    text.push_str(source);
                    if !source.ends_with('\n') {
                        text.push('\n');
                    }
                }
            }
        }
        text
    }
}
fn column(
    role: Role,
    version: Version,
    show_mode: bool,
    selected: &std::path::Path,
    cancel: &Cancel,
) -> anyhow::Result<Column> {
    let Version::Present {
        path,
        oid,
        mode,
        source,
    } = version
    else {
        return Ok(Column {
            role,
            description: i18n::tr!(ConflictAbsent).into(),
            body: Body::Notice(i18n::tr!(ConflictAbsent).into()),
        });
    };
    let limited = matches!(&source, Source::Text { text, .. } if text.lines().count() > 50_000);
    let (body, syntax) = prepare_source(source, limited, cancel)?;
    let mut description = oid.to_string()[..12].to_owned();
    if show_mode {
        description.push_str(&format!(" · {mode:o}"));
    }
    if let Some(syntax) = syntax {
        description.push_str(&format!(" · {syntax}"));
    }
    // 不同阶段可以指向不同路径（例如重命名冲突），保留实际索引路径。
    if path != selected {
        description.push_str(&format!(" · {}", path.display()));
    }
    Ok(Column {
        role,
        description,
        body,
    })
}

fn prepare_source(
    source: Source,
    limited: bool,
    cancel: &Cancel,
) -> anyhow::Result<(Body, Option<String>)> {
    let requested: BTreeSet<_> = match &source {
        Source::Text { text, .. } => (1..=text.lines().count().min(50_000))
            .filter_map(|n| NonZeroU32::new(n as u32))
            .collect(),
        _ => BTreeSet::new(),
    };
    let paint = diff::source::prepare(&source, &requested, &[], diff::source::Side::Old, cancel)?;
    let syntax = paint.label();
    let body = match source {
        Source::Text { text, .. } if !limited => {
            let digits = text.lines().count().to_string().len().max(2);
            let mut lines = Vec::new();
            for (index, source) in text.split_inclusive('\n').enumerate() {
                cancel.check()?;
                let number = NonZeroU32::new(index as u32 + 1).expect("bounded line number");
                let mut spans = vec![Span::styled(
                    format!("{:>digits$} │ ", index + 1),
                    Style::default().fg(Color::DarkGray),
                )];
                spans.extend(diff::source::spans(
                    source,
                    paint.line(number),
                    Style::default(),
                ));
                let mut line = links::Line::from(Line::from(spans));
                line.detect(&[]);
                lines.push(line);
            }
            Body::Text {
                lines,
                gutter: digits + 3,
                source: text.into(),
            }
        }
        Source::Text { .. } => Body::Notice(i18n::tr!(ConflictLineLimit).into()),
        Source::Absent => Body::Notice(i18n::tr!(ConflictAbsent).into()),
        Source::Image(_) | Source::Unavailable(_) => {
            Body::Notice(paint.label().expect("omitted source label"))
        }
    };
    Ok((body, syntax))
}
