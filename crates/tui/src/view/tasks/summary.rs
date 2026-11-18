//! Task counts carry semantic styles in each span, including when composed into other lines.
use super::super::{ACCENT, MUTED, WARN};
use ratatui::{
    style::Style,
    text::{Line, Span},
};

pub(in crate::view) fn counts(failures: usize, running: usize) -> Line<'static> {
    let mut line = Line::default();
    if failures > 0 {
        line.spans.push(Span::styled(
            i18n::tf!(TaskUnreadFailuresCount, failures = failures),
            Style::default().fg(WARN),
        ));
    }
    if running > 0 {
        if failures > 0 {
            line.spans
                .push(Span::styled(" · ", Style::default().fg(MUTED)));
        }
        line.spans.push(Span::styled(
            i18n::tf!(TaskRunningCount, running = running),
            Style::default().fg(ACCENT),
        ));
    }
    line
}
pub(in crate::view) fn entry() -> Line<'static> {
    Line::from(vec![
        Span::styled("/tasks", Style::default().fg(MUTED)),
        Span::styled(i18n::tr!(TasksEntrySuffix), Style::default().fg(MUTED)),
    ])
}
pub(in crate::view) fn footer(failures: usize, running: usize) -> Line<'static> {
    let mut line = counts(failures, running);
    if !line.spans.is_empty() {
        line.spans
            .push(Span::styled(" · ", Style::default().fg(MUTED)));
    }
    line.spans.extend(entry().spans);
    line
}
