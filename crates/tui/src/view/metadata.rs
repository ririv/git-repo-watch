//! Shared metadata fields keep terminal-safe values and local timestamps consistent.
use super::*;
pub(super) fn time(seconds: i64) -> String {
    match chrono::DateTime::from_timestamp(seconds, 0) {
        Some(time) => time
            .with_timezone(&chrono::Local)
            .format("%Y-%m-%d %H:%M:%S")
            .to_string(),
        None => format!("{}: {seconds}", i18n::tr!(HistoryInvalidTime)),
    }
}

pub(super) fn field(key: &str, value: impl Into<String>) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{key}: "), Style::default().fg(MUTED)),
        Span::raw(safe(&value.into())),
    ])
}
