//! Render upstream comparisons with the same colors across repository views.
use super::{BAD, GOOD, MUTED, WARN, safe};
use engine::local::branches::upstream::{Comparison, Tracking};
use ratatui::{
    style::Style,
    text::{Line, Span},
};

pub(super) fn commit_counts(ahead: usize, behind: usize) -> [Span<'static>; 3] {
    let count = |arrow, commits| {
        Span::styled(
            format!("{arrow}{commits}"),
            Style::default().fg(if commits == 0 { GOOD } else { WARN }),
        )
    };
    [count('↑', ahead), Span::raw(" "), count('↓', behind)]
}

pub(super) fn upstream(tracking: &Result<Tracking, String>) -> Line<'static> {
    let mut spans = vec![Span::styled(
        i18n::tf!(FieldLabel, label = i18n::tr!(OverviewUpstream)),
        Style::default().fg(MUTED),
    )];
    match tracking {
        Ok(Tracking::Absent) => spans.push(Span::styled(
            i18n::tr!(NoUpstream),
            Style::default().fg(MUTED),
        )),
        Ok(Tracking::Local { name, comparison } | Tracking::Remote { name, comparison }) => {
            spans.push(Span::raw(format!("{} · ", safe(name))));
            match comparison {
                Comparison::Known { ahead, behind } => spans.extend(commit_counts(*ahead, *behind)),
                Comparison::Missing => spans.push(Span::raw(i18n::tr!(OverviewGone))),
            }
        }
        Err(error) => spans.push(Span::styled(safe(error), Style::default().fg(BAD))),
    }
    Line::from(spans)
}
