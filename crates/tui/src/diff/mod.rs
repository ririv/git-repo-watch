//! Cached logical diff rows shared by responsive unified and side-by-side layouts.
mod build;
mod layout;
#[cfg(test)]
mod layout_tests;
mod source;
#[cfg(test)]
mod tests;
mod text;
mod wrap;
use crate::view::safe;
use anyhow::Result;
use engine::{
    cancel::Cancel,
    local::files::{
        Area,
        patch::{Document, Extent, Line as PatchLine, Metadata},
    },
};
pub(super) use layout::{Cache, VisualRow};
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
impl Preference {
    pub const ALL: [Self; 3] = [Self::Auto, Self::Split, Self::Unified];
    pub fn advance(self, backwards: bool) -> Self {
        match (self, backwards) {
            (Self::Auto, false) | (Self::Unified, true) => Self::Split,
            (Self::Split, false) | (Self::Auto, true) => Self::Unified,
            (Self::Unified, false) | (Self::Split, true) => Self::Auto,
        }
    }
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
#[derive(Debug)]
struct Code {
    number: NonZeroU32,
    content: Line<'static>,
}
#[derive(Debug)]
enum Row {
    Metadata(Line<'static>),
    Context { old: Code, new: Code },
    Added(Code),
    Removed(Code),
}
impl Row {
    fn is_edit(&self) -> bool {
        matches!(self, Self::Added(_) | Self::Removed(_))
    }
    fn max_number(&self) -> u32 {
        match self {
            Self::Context { old, new } => old.number.get().max(new.number.get()),
            Self::Added(code) | Self::Removed(code) => code.number.get(),
            Self::Metadata(_) => 1,
        }
    }
}
#[derive(Debug)]
pub(super) struct View {
    pub(crate) export: Export,
    rows: Vec<Row>,
    digits: u16,
    comparison: Comparison,
    cache: Option<Cache>,
}
#[derive(Debug)]
pub(crate) enum Export {
    Complete(std::sync::Arc<str>),
    Unavailable,
    Truncated,
}
impl View {
    pub fn selected_layout(&self, width: u16, preference: Preference) -> Layout {
        preference.resolve(width, self.digits)
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
        scroll: &mut usize,
    ) -> &mut Cache {
        let mode = preference.resolve(width, self.digits);
        if self
            .cache
            .as_ref()
            .is_none_or(|cache| cache.width != width || cache.mode != mode)
        {
            let anchor = self.cache.as_ref().and_then(|cache| cache.anchor(*scroll));
            let mut cache = layout::arrange(&self.rows, self.digits, width, mode);
            if let Some(anchor) = anchor {
                *scroll = cache.locate(anchor);
                cache.restore(anchor, *scroll);
            }
            self.cache = Some(cache);
        }
        self.cache.as_mut().expect("diff layout prepared")
    }
    pub fn labels(&self) -> (String, String) {
        self.comparison.labels()
    }
}

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

fn prepare_comparison(
    title: &str,
    document: Document,
    comparison: Comparison,
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
    let rows = build::rows(title, document, cancel)?;
    let digits = rows
        .iter()
        .map(Row::max_number)
        .max()
        .unwrap_or(1)
        .to_string()
        .len() as u16;
    Ok(Box::new(View {
        export,
        rows,
        digits,
        comparison,
        cache: None,
    }))
}
