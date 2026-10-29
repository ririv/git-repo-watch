//! A checkbox owns its three cells; surrounding layouts own their spacing.
use super::*;
use crate::files::CheckState;

pub(super) const WIDTH: u16 = 3;

pub(super) fn spans(state: CheckState) -> Vec<Span<'static>> {
    let mark = match state {
        CheckState::Unchecked => Span::raw(" "),
        CheckState::Mixed => Span::styled("−", Style::default().fg(WARN).bold()),
        CheckState::Checked => {
            Span::styled("✓", Style::default().fg(Color::Rgb(73, 199, 140)).bold())
        }
    };
    vec![
        Span::styled("[", Style::default().fg(MUTED)),
        mark,
        Span::styled("]", Style::default().fg(MUTED)),
    ]
}
