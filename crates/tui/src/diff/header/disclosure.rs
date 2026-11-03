//! Expand the original header without losing the code viewport or cached layout.
use super::super::{Cache, MUTED, Row, View};
use ratatui::{style::Style, text::Line};

#[derive(Debug, Default)]
pub(crate) enum Raw {
    #[default]
    Absent,
    Collapsed(Vec<String>),
    Expanded {
        lines: Vec<String>,
        position: Box<Position>,
    },
}

#[derive(Debug)]
pub(crate) struct Position {
    cache: Option<Cache>,
    scroll: usize,
    horizontal: crate::horizontal::Viewport,
}

impl Raw {
    pub fn expanded(&self) -> bool {
        matches!(self, Self::Expanded { .. })
    }
    pub fn new(lines: Vec<String>) -> Self {
        if lines.is_empty() {
            Self::Absent
        } else {
            Self::Collapsed(lines)
        }
    }
    pub fn lines(&self) -> &[String] {
        match self {
            Self::Absent => &[],
            Self::Collapsed(lines) | Self::Expanded { lines, .. } => lines,
        }
    }
}

impl View {
    pub fn set_header(&mut self, visibility: super::super::HeaderVisibility, scroll: &mut usize) {
        if self.header.expanded() != (visibility == super::super::HeaderVisibility::Expanded) {
            self.toggle_header(scroll);
        }
    }
    pub fn toggle_header(&mut self, scroll: &mut usize) -> bool {
        self.header = match std::mem::take(&mut self.header) {
            Raw::Absent => return false,
            Raw::Collapsed(lines) => {
                let position = Box::new(Position {
                    cache: self.cache.take(),
                    scroll: *scroll,
                    horizontal: std::mem::take(&mut self.horizontal),
                });
                let start = self.header_start();
                self.rows.splice(
                    start..start,
                    lines.iter().map(|line| {
                        Row::Metadata(Line::styled(
                            crate::view::safe(line),
                            Style::default().fg(MUTED),
                        ))
                    }),
                );
                *scroll = 0;
                if let super::super::Interaction::Editable(Some(marking)) = &mut self.interaction {
                    marking.reveal = false;
                }
                Raw::Expanded { lines, position }
            }
            Raw::Expanded { lines, position } => {
                let start = self.header_start();
                self.rows.drain(start..start + lines.len());
                self.cache = position.cache;
                self.horizontal = position.horizontal;
                *scroll = position.scroll;
                Raw::Collapsed(lines)
            }
        };
        true
    }
}

impl crate::Screen {
    pub(crate) fn set_diff_header(&mut self, visibility: super::super::HeaderVisibility) {
        self.diff_header = visibility;
        self.save_view_preference(engine::state::view::Edit::DiffHeader(self.diff_header));
    }
    pub(crate) fn diff_header_label(&self) -> &'static str {
        match self.diff_header {
            super::super::HeaderVisibility::Collapsed => i18n::tr!(DiffHeaderCollapsed),
            super::super::HeaderVisibility::Expanded => i18n::tr!(DiffHeaderExpanded),
        }
    }
}
