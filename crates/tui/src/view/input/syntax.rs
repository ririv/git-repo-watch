//! Color command roles inside the editor's existing UTF-8 viewport.
use super::{Span, Style};
use crate::{
    command::{
        Input,
        syntax::{self, Role},
    },
    view::{ACCENT, WARN},
};
use ratatui::style::Color;
use std::ops::Range;

pub(super) fn spans(input: &Input, visible: Range<usize>) -> Vec<Span<'static>> {
    syntax::segments(input)
        .into_iter()
        .filter_map(|segment| {
            let start = segment.range.start.max(visible.start);
            let end = segment.range.end.min(visible.end);
            if start >= end {
                return None;
            }
            let style = style(segment.role);
            Some(Span::styled(
                input.editor().text[start..end].to_owned(),
                style,
            ))
        })
        .collect()
}

pub(super) fn style(role: Role) -> Style {
    match role {
        Role::Command => Style::default().fg(ACCENT).bold(),
        Role::Argument => Style::default().fg(Color::LightCyan),
        Role::Option => Style::default().fg(WARN).bold(),
    }
}
