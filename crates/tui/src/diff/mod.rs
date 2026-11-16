//! Cached logical diff rows shared by responsive unified and side-by-side layouts.
mod build;
#[cfg(test)]
mod context_tests;
mod full;
mod position;
pub(crate) use position::Position;
mod header;
pub(crate) use engine::state::view::{DiffContext as Context, HeaderVisibility};
pub(crate) fn context_label(context: Context) -> &'static str {
    match context {
        Context::Full => i18n::tr!(DiffContextFull),
        Context::Changes => i18n::tr!(DiffContextChanges),
    }
}
pub(crate) use header::summary as header_summary;
mod layout;
#[cfg(test)]
mod layout_tests;
#[cfg(test)]
mod newline_tests;
pub(crate) mod notice;
pub(crate) mod source;
#[cfg(test)]
mod tests;
mod text;
use crate::view::safe;
use anyhow::Result;
#[cfg(test)]
use engine::local::files::Area;
use engine::{
    cancel::Cancel,
    local::files::patch::{Document, Extent, Line as PatchLine, Metadata},
};
pub(super) use layout::{Anchor, Anchors, Cache, RowKind, VisualRow};
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};
use std::{collections::BTreeSet, num::NonZeroU32};

pub(super) const ADDED: Color = Color::Rgb(28, 51, 43);
pub(super) const REMOVED: Color = Color::Rgb(58, 32, 41);
const MUTED: Color = Color::Rgb(125, 140, 155);
const MIN_SPLIT_WIDTH: u16 = 140;
const MIN_AUTO_CODE_WIDTH: u16 = 60;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Preference {
    #[default]
    Auto,
    Split,
    Unified,
}
#[derive(Debug, Clone, Copy)]
pub(crate) struct Options {
    pub layout: Preference,
    pub wrap: bool,
    pub header: HeaderVisibility,
    pub context: Context,
}
impl Preference {
    pub fn label(self) -> &'static str {
        match self {
            Self::Auto => i18n::tr!(DiffAuto),
            Self::Split => i18n::tr!(DiffSplit),
            Self::Unified => i18n::tr!(DiffUnified),
        }
    }
    fn resolve(self, width: u16, digits: u16) -> Layout {
        match self {
            Self::Unified => Layout::Unified,
            Self::Split => Layout::Split,
            Self::Auto => {
                let column = width.saturating_sub(1) / 2;
                if width >= MIN_SPLIT_WIDTH
                    && column.saturating_sub(digits + 3) >= MIN_AUTO_CODE_WIDTH
                {
                    Layout::Split
                } else {
                    Layout::Unified
                }
            }
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Layout {
    Unified,
    Split,
}
impl Layout {
    pub fn label(self) -> &'static str {
        match self {
            Self::Unified => i18n::tr!(DiffUnified),
            Self::Split => i18n::tr!(DiffSplit),
        }
    }
}
#[derive(Debug, Clone, Copy)]
pub(super) enum Comparison {
    Staged,
    Working,
    Conflict,
    Stash(engine::local::stash::preview::Layer),
    History {
        before: engine::local::history::Base,
        after: engine::local::history::Oid,
    },
}
impl Comparison {
    pub(super) fn labels(self) -> (String, String) {
        match self {
            Self::Staged => ("HEAD".into(), i18n::tr!(Index).into()),
            Self::Working => (i18n::tr!(Index).into(), i18n::tr!(Worktree).into()),
            Self::Conflict => ("HEAD".into(), i18n::tr!(ReplayConflictWorktree).into()),
            Self::Stash(layer) => {
                let (before, after) = layer.labels();
                (before.into(), after.into())
            }
            Self::History { before, after } => (
                match before {
                    engine::local::history::Base::Empty => i18n::tr!(HistoryRoot).into(),
                    engine::local::history::Base::Commit(oid) => oid.to_string()[..12].into(),
                },
                after.to_string()[..12].into(),
            ),
        }
    }
}
#[derive(Debug, Clone)]
struct Code {
    number: NonZeroU32,
    content: Line<'static>,
    ending: Ending,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Ending {
    Newline,
    EndOfFile,
}
#[derive(Debug, Clone)]
enum Row {
    Information(Line<'static>),
    Metadata(Line<'static>),
    Context { old: Code, new: Code },
    Added(Code),
    Removed(Code),
}
impl Row {
    pub fn line_id(&self) -> Option<engine::local::files::partial::LineId> {
        use engine::local::files::partial::LineId;
        match self {
            Self::Added(code) => Some(LineId::New(code.number)),
            Self::Removed(code) => Some(LineId::Old(code.number)),
            _ => None,
        }
    }
    fn is_edit(&self) -> bool {
        matches!(self, Self::Added(_) | Self::Removed(_))
    }
    fn max_number(&self) -> u32 {
        match self {
            Self::Context { old, new } => old.number.get().max(new.number.get()),
            Self::Added(code) | Self::Removed(code) => code.number.get(),
            Self::Information(_) | Self::Metadata(_) => 1,
        }
    }
}
#[derive(Debug)]
pub(super) struct View {
    pub interaction: Interaction,
    pub(crate) export: Export,
    pub information: Option<Information>,
    pub header: header::Raw,
    rows: Vec<Row>,
    full: Result<Vec<Row>, &'static str>,
    reading: Option<Position>,
    digits: u16,
    comparison: Comparison,
    cache: Option<Cache>,
    pub horizontal: crate::horizontal::Viewport,
}
#[derive(Debug)]
pub(super) struct Information {
    pub title: String,
    pub syntax: Vec<String>,
    pub facts: Vec<header::Fact>,
}
#[derive(Debug)]
pub(crate) enum Interaction {
    ReadOnly,
    Editable(Option<Marking>),
}
#[derive(Debug, Clone)]
pub(crate) struct Marking {
    pub label: String,
    pub lines: BTreeSet<engine::local::files::partial::LineId>,
    pub cursor: engine::local::files::partial::LineId,
    pub reveal: bool,
}
#[derive(Debug)]
pub(crate) enum Export {
    Complete(std::sync::Arc<str>),
    Unavailable,
    Truncated,
}
impl View {
    pub fn select(
        &mut self,
        selection: Option<(
            BTreeSet<engine::local::files::partial::LineId>,
            engine::local::files::partial::LineId,
            String,
        )>,
    ) {
        let Interaction::Editable(marking) = &mut self.interaction else {
            return;
        };
        match selection {
            None => *marking = None,
            Some((lines, cursor, label)) => {
                if !marking.as_ref().is_some_and(|current| {
                    current.lines == lines && current.cursor == cursor && current.label == label
                }) {
                    *marking = Some(Marking {
                        label,
                        lines,
                        cursor,
                        reveal: true,
                    });
                }
            }
        }
    }
    pub fn layout(
        &self,
        width: u16,
        preference: Preference,
        wrap: bool,
        paths: &[std::path::PathBuf],
        header: HeaderVisibility,
        context: Context,
    ) -> Cache {
        let (mut rows, current) = match context {
            Context::Changes => (self.rows.clone(), self.header.expanded()),
            Context::Full => (
                match &self.full {
                    Ok(rows) => rows.clone(),
                    Err(reason) => vec![Row::Information(Line::styled(
                        *reason,
                        Style::default().fg(Color::Yellow),
                    ))],
                },
                false,
            ),
        };
        let wanted = header == HeaderVisibility::Expanded;
        if current != wanted && !self.header.lines().is_empty() {
            let raw = self.header.lines();
            let start = rows
                .iter()
                .take_while(|row| matches!(row, Row::Information(_)))
                .count();
            if wanted {
                rows.splice(
                    start..start,
                    raw.iter().map(|line| {
                        Row::Metadata(Line::styled(safe(line), Style::default().fg(MUTED)))
                    }),
                );
            } else {
                rows.drain(start..start + raw.len());
            }
        }
        let mut cache = layout::arrange(
            &rows,
            self.digits,
            width,
            preference.resolve(width, self.digits),
            wrap,
            paths,
        );
        cache.context = context;
        cache
    }
    pub fn header_rows(&self) -> usize {
        self.header.lines().len()
    }
    pub fn header_start(&self) -> usize {
        self.rows
            .iter()
            .take_while(|row| matches!(row, Row::Information(_)))
            .count()
    }

    pub fn selected_layout(&self, width: u16, preference: Preference) -> Layout {
        preference.resolve(width, self.digits)
    }
    pub fn gutter_width(&self, mode: Layout) -> u16 {
        match mode {
            Layout::Unified => 2 * self.digits + 4,
            Layout::Split => self.digits + 3,
        }
    }
    pub fn minimum_width(&self, mode: Layout) -> u16 {
        match mode {
            Layout::Unified => 2 * self.digits + 6,
            Layout::Split => 2 * (self.digits + 5) + 1,
        }
    }
    pub fn arrange(
        &mut self,
        width: u16,
        preference: Preference,
        wrap: bool,
        paths: &[std::path::PathBuf],
        scroll: &mut usize,
        context: Context,
    ) -> &mut Cache {
        let mode = preference.resolve(width, self.digits);
        if self.cache.as_ref().is_none_or(|cache| {
            cache.width != width
                || cache.mode != mode
                || cache.wrap != wrap
                || !cache.matches_paths(paths)
                || cache.context != context
        }) {
            let changed_context = self
                .cache
                .as_ref()
                .is_some_and(|cache| cache.context != context);
            let anchor = self.cache.as_ref().and_then(|cache| cache.anchor(*scroll));
            if changed_context {
                self.reading = self.position(*scroll);
            }
            let mut cache = self.layout(
                width,
                preference,
                wrap,
                paths,
                if self.header.expanded() {
                    HeaderVisibility::Expanded
                } else {
                    HeaderVisibility::Collapsed
                },
                context,
            );
            if !changed_context && let Some(anchor) = anchor {
                *scroll = cache.locate(anchor);
                cache.restore(anchor, *scroll);
            }
            self.cache = Some(cache);
            if let Interaction::Editable(Some(marking)) = &mut self.interaction
                && !self.header.expanded()
            {
                marking.reveal = true;
            }
        }
        self.restore_reading(scroll);
        let cache = self.cache.as_mut().expect("diff layout prepared");
        self.horizontal.constrain(cache.horizontal_maximum());
        cache
    }
    pub fn labels(&self) -> (String, String) {
        self.comparison.labels()
    }
}

#[cfg(test)]
pub(super) fn prepare(
    title: &str,
    document: Document,
    area: Area,
    cancel: &Cancel,
) -> Result<Box<View>> {
    prepare_comparison(
        title,
        document,
        match area {
            Area::Staged => Comparison::Staged,
            Area::Unstaged | Area::Untracked => Comparison::Working,
        },
        cancel,
    )
}

#[cfg(test)]
pub(super) fn prepare_history(
    title: &str,
    document: Document,
    before: engine::local::history::Base,
    after: engine::local::history::Oid,
    cancel: &Cancel,
) -> Result<Box<View>> {
    prepare_comparison(
        title,
        document,
        Comparison::History { before, after },
        cancel,
    )
}

#[cfg(test)]
pub(crate) fn prepare_inline_comparison(
    document: Document,
    comparison: Comparison,
    cancel: &Cancel,
) -> Result<Box<View>> {
    prepare_inline_with_theme(document, comparison, highlight::Theme::Dark, cancel)
}

pub(crate) fn prepare_inline_with_theme(
    document: Document,
    comparison: Comparison,
    theme: highlight::Theme,
    cancel: &Cancel,
) -> Result<Box<View>> {
    prepare_rows(build::Heading::Inline, document, comparison, theme, cancel)
}

#[cfg(test)]
pub(super) fn prepare_comparison(
    title: &str,
    document: Document,
    comparison: Comparison,
    cancel: &Cancel,
) -> Result<Box<View>> {
    prepare_with_theme(title, document, comparison, highlight::Theme::Dark, cancel)
}

pub(crate) fn prepare_with_theme(
    title: &str,
    document: Document,
    comparison: Comparison,
    theme: highlight::Theme,
    cancel: &Cancel,
) -> Result<Box<View>> {
    prepare_rows(
        build::Heading::Full(title),
        document,
        comparison,
        theme,
        cancel,
    )
}

fn prepare_rows(
    heading: build::Heading<'_>,
    document: Document,
    comparison: Comparison,
    theme: highlight::Theme,
    cancel: &Cancel,
) -> Result<Box<View>> {
    let export = match &document {
        Document::Notice(_) => Export::Unavailable,
        Document::Patch {
            extent: engine::local::files::patch::Extent::Truncated,
            ..
        } => Export::Truncated,
        Document::Patch { .. } => Export::Complete(document.text().into()),
    };
    let build::Prepared {
        rows,
        information,
        header,
        full,
    } = build::rows(heading, document, theme, cancel)?;
    let digits = rows
        .iter()
        .chain(full.as_ref().into_iter().flatten())
        .map(Row::max_number)
        .max()
        .unwrap_or(1)
        .to_string()
        .len() as u16;
    Ok(Box::new(View {
        interaction: Interaction::ReadOnly,
        export,
        information,
        header: header::Raw::new(header),
        rows,
        full,
        reading: None,
        digits,
        comparison,
        cache: None,
        horizontal: Default::default(),
    }))
}
