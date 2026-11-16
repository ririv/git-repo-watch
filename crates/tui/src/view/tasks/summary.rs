//! Task counts carry semantic styles in each span, including when composed into other lines.
use super::super::{ACCENT, MUTED, WARN};
use ratatui::{
    style::Style,
    text::{Line, Span},
};

pub(in crate::view) fn counts(pending: usize, running: usize) -> Line<'static> {
    let mut line = Line::from(Span::styled(
        i18n::tf!(TaskFailuresCount, pending = pending),
        Style::default().fg(if pending > 0 { WARN } else { MUTED }),
    ));
    if running > 0 {
        line.spans.extend([
            Span::styled(" · ", Style::default().fg(MUTED)),
            Span::styled(
                i18n::tf!(TaskRunningCount, running = running),
                Style::default().fg(ACCENT),
            ),
        ]);
    }
    line
}
pub(in crate::view) fn entry() -> Line<'static> {
    Line::from(vec![
        Span::styled("/tasks", Style::default().fg(MUTED)),
        Span::styled(i18n::tr!(TasksEntrySuffix), Style::default().fg(MUTED)),
    ])
}
pub(in crate::view) fn footer(pending: usize, running: usize) -> Line<'static> {
    let mut line = if pending + running > 0 {
        counts(pending, running)
    } else {
        Line::default()
    };
    if !line.spans.is_empty() {
        line.spans
            .push(Span::styled(" · ", Style::default().fg(MUTED)));
    }
    line.spans.extend(entry().spans);
    line
}
