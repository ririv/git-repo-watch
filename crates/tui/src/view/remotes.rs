//! Remote addresses share one presentation across live and cached repository details.
use super::*;
use crate::remote_info::Content;
use engine::local::remotes::Direction;

pub(super) fn lines(screen: &Screen, path: &std::path::Path) -> Vec<Line<'static>> {
    let label = i18n::tr!(RemoteRepositories);
    let field = |value: String, color| {
        Line::from(vec![
            Span::styled(
                i18n::tf!(FieldLabel, label = label),
                Style::default().fg(MUTED),
            ),
            Span::styled(value, Style::default().fg(color)),
        ])
    };
    match screen.remote_info.content(path) {
        Some(Content::Ready {
            result: Ok(remotes),
            ..
        }) if remotes.is_empty() => vec![field(i18n::tr!(RemoteNone).into(), MUTED)],
        Some(Content::Ready {
            result: Ok(remotes),
            ..
        }) => {
            let mut lines = vec![field(String::new(), Color::Reset)];
            lines.extend(remotes.iter().map(|remote| {
                let direction = match remote.direction {
                    Direction::Fetch => String::new(),
                    Direction::Push => format!(" ({})", i18n::tr!(RemotePush)),
                };
                Line::from(vec![
                    Span::styled(
                        format!("  {}{direction} · ", safe(&remote.name)),
                        Style::default().fg(MUTED),
                    ),
                    Span::raw(safe(&remote.url)),
                ])
            }));
            lines
        }
        Some(Content::Ready {
            result: Err(error), ..
        }) => vec![field(safe(error), BAD)],
        Some(Content::Loading(_)) | Some(Content::Closed) | None => {
            vec![field(i18n::tr!(OverviewLoading).into(), MUTED)]
        }
    }
}
