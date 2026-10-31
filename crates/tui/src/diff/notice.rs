//! Render patch notices separately from source code while preserving the raw patch.
use engine::local::files::patch::BinaryPath;
use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
};

pub(super) const NO_NEWLINE: &str = "⊖";

pub(super) fn ending(gutter: usize) -> crate::links::Line {
    Line::from(vec![
        Span::raw(" ".repeat(gutter)),
        Span::styled(NO_NEWLINE, Style::default().fg(Color::LightRed)),
    ])
    .into()
}

pub(super) fn binary(before: &BinaryPath, after: &BinaryPath) -> String {
    let path = |side: &BinaryPath, prefix| match side {
        BinaryPath::Absent => "/dev/null".to_owned(),
        BinaryPath::File(path) => format!("{prefix}/{}", super::safe(&path.display().to_string())),
    };
    format!(
        "Binary files {} and {} differ",
        path(before, "a"),
        path(after, "b")
    )
}
