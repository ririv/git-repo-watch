//! A working result is source text, while non-text documents keep their existing renderer.
use super::{Body, Reveal};
use crate::{diff, document, horizontal, links};
use engine::{
    cancel::Cancel,
    local::files::{
        Change,
        patch::{Document, Source},
    },
};

#[derive(Debug)]
pub(crate) enum Content {
    Source { body: Body, syntax: String },
    Document(Box<document::View>),
}
#[derive(Debug)]
pub(crate) struct Row {
    pub line: links::Line,
    pub kind: diff::RowKind,
}
#[derive(Debug)]
pub(crate) struct Cache {
    pub width: u16,
    pub wrap: bool,
    pub rows: Vec<Row>,
    pub anchors: Vec<usize>,
}
#[derive(Debug)]
pub(crate) struct View {
    pub content: Content,
    pub horizontal: horizontal::Viewport,
    pub cache: Option<Cache>,
    reveal: Reveal,
}
impl View {
    pub fn prepare(
        source: Source,
        document: Document,
        change: &Change,
        theme: highlight::Theme,
        cancel: &Cancel,
    ) -> anyhow::Result<Self> {
        let content = match source {
            source @ Source::Text { .. } => {
                let limited =
                    matches!(&source, Source::Text {text,..} if text.lines().count() > 50_000);
                let (body, syntax) = super::prepare_source(source, limited, theme, cancel)?;
                Content::Source {
                    body,
                    syntax: syntax.expect("text source syntax label"),
                }
            }
            _ => Content::Document(document::prepare_change_with_theme(
                &change.location.display(),
                document,
                change,
                theme,
                cancel,
            )?),
        };
        Ok(Self {
            content,
            horizontal: Default::default(),
            cache: None,
            reveal: Reveal::FirstChange,
        })
    }
    pub fn source(&self) -> Option<&str> {
        match &self.content {
            Content::Source {
                body: Body::Text { source, .. },
                ..
            } => Some(source),
            _ => None,
        }
    }
    pub fn seek(&mut self, line: usize, scroll: &mut usize) {
        if let Some(cache) = &self.cache {
            *scroll = cache
                .anchors
                .iter()
                .position(|at| *at >= line)
                .unwrap_or(cache.anchors.len());
        }
        self.reveal = Reveal::At(line);
        self.cache = None;
    }
    pub fn horizontal(&mut self) -> Option<&mut horizontal::Viewport> {
        match &mut self.content {
            Content::Source { .. } => Some(&mut self.horizontal),
            Content::Document(view) => view.horizontal(),
        }
    }
    pub fn offset(&self) -> usize {
        match &self.content {
            Content::Source { .. } => self.horizontal.offset(),
            Content::Document(view) => view.text().map_or(0, |v| v.horizontal.offset()),
        }
    }
    pub fn restore_offset(&mut self, offset: usize) {
        if let Some(horizontal) = self.horizontal() {
            horizontal.constrain(offset);
            horizontal.seek(offset);
        }
    }
    pub fn retain(&mut self) {
        self.reveal = Reveal::Retain;
    }
    pub fn arrange(&mut self, width: u16, wrap: bool, visible: usize, scroll: &mut usize) {
        if self
            .cache
            .as_ref()
            .is_some_and(|c| c.width == width && c.wrap == wrap)
        {
            return;
        }
        let anchor = self
            .cache
            .as_ref()
            .and_then(|c| c.anchors.get(*scroll))
            .copied();
        let Content::Source { body, .. } = &self.content else {
            return;
        };
        let (lines, gutter, first) = match body {
            Body::Text { lines, gutter, .. } if !lines.is_empty() => (
                lines.clone(),
                *gutter,
                lines
                    .iter()
                    .position(|l| l.to_string().contains("<<<<<<<"))
                    .unwrap_or(0),
            ),
            Body::Text { .. } => (
                vec![links::Line::from(ratatui::text::Line::from(i18n::tr!(
                    ConflictEmpty
                )))],
                0,
                0,
            ),
            Body::Notice(text) => (
                vec![links::Line::from(ratatui::text::Line::from(text.clone()))],
                0,
                0,
            ),
        };
        let mut rows = Vec::new();
        let mut anchors = Vec::new();
        let mut maximum = 0;
        for (index, line) in lines.iter().enumerate() {
            maximum = maximum.max(if wrap {
                0
            } else {
                line.width().saturating_sub(width as usize)
            });
            let parts = if wrap {
                line.wrap_pinned(width as usize, gutter)
            } else {
                vec![line.clone()]
            };
            for part in parts {
                rows.push(Row {
                    line: part,
                    kind: diff::RowKind::Content,
                });
                anchors.push(index);
            }
        }
        if let Body::Text {
            source,
            lines,
            gutter,
        } = body
            && !source.is_empty()
            && !source.ends_with('\n')
        {
            rows.push(Row {
                line: diff::notice::ending(*gutter),
                kind: diff::RowKind::Annotation,
            });
            anchors.push(lines.len().saturating_sub(1));
        }
        self.horizontal.constrain(maximum);
        let target = match (anchor, self.reveal) {
            (_, Reveal::At(line)) => Some(line),
            (Some(n), _) => Some(n),
            (None, Reveal::FirstChange) => {
                Some(first.saturating_sub(visible.saturating_sub(1).min(3)))
            }
            _ => None,
        };
        if let Some(target) = target {
            *scroll = anchors.iter().position(|n| *n == target).unwrap_or(0);
        }
        self.reveal = Reveal::Retain;
        self.cache = Some(Cache {
            width,
            wrap,
            rows,
            anchors,
        });
    }
    pub fn gutter(&self) -> usize {
        match &self.content {
            Content::Source {
                body: Body::Text { gutter, .. },
                ..
            } => *gutter,
            _ => 0,
        }
    }
}
