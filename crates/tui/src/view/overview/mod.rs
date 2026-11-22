//! Compact live facts complement the existing repository check overview.
use super::*;
use crate::overview::Content;
use engine::local::overview::{Checkout, Operation};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Presentation {
    Expanded,
    Embedded,
}
pub(super) fn render(frame: &mut Frame, screen: &mut Screen, area: Rect) {
    draw(frame, screen, area, Presentation::Expanded);
}
pub(super) fn panel(frame: &mut Frame, screen: &mut Screen, area: Rect) {
    draw(frame, screen, area, Presentation::Embedded);
}
fn draw(frame: &mut Frame, screen: &mut Screen, area: Rect, presentation: Presentation) {
    let path = screen.workspace.path();
    let repository_title = path
        .map(name)
        .unwrap_or_else(|| i18n::tr!(NoRepositorySelected).into());
    let title = if presentation == Presentation::Embedded {
        i18n::tr!(WorkspaceOverview).to_owned()
    } else {
        repository_title
    };
    let mut lines = Vec::new();
    if presentation == Presentation::Expanded
        && let Some(path) = path
    {
        lines.push(field(
            i18n::tr!(Path),
            safe(&path.to_string_lossy()),
            Color::Reset,
        ));
    }
    match &screen.overview.content {
        Content::Loading(_) => lines.push(Line::from(format!(
            "{} {}",
            screen.overview.spinner(),
            i18n::tr!(OverviewLoading)
        ))),
        Content::Failed { error, .. } => lines.extend(
            multiline(error)
                .lines()
                .map(|line| Line::styled(line.to_owned(), Style::default().fg(BAD))),
        ),
        Content::Ready(snapshot) | Content::Refreshing(snapshot) => {
            match &snapshot.checkout {
                Ok(Checkout::Branch {
                    name,
                    oid,
                    tracking,
                }) => {
                    lines.push(field(
                        i18n::tr!(Branch),
                        format!("{}  {}", safe(name), &oid.to_string()[..8]),
                        ACCENT,
                    ));
                    lines.push(synchronization::upstream(tracking));
                }
                Ok(Checkout::Detached(oid)) => lines.push(field(
                    i18n::tr!(HeadState),
                    format!("{} {}", i18n::tr!(OverviewDetached), &oid.to_string()[..8]),
                    WARN,
                )),
                Ok(Checkout::Unborn(branch)) => lines.push(field(
                    i18n::tr!(Branch),
                    format!("{} · {}", safe(branch), i18n::tr!(OverviewUnborn)),
                    ACCENT,
                )),
                Err(error) => lines.push(field(i18n::tr!(OverviewCheckout), safe(error), BAD)),
            }
            if snapshot.depth == engine::local::history::Depth::Shallow {
                lines.push(Line::styled(
                    i18n::tr!(OverviewShallow),
                    Style::default().fg(WARN),
                ));
            }
            match &snapshot.changes {
                Ok(changes) => {
                    let values = [
                        (i18n::tr!(OverviewStaged), changes.staged),
                        (i18n::tr!(OverviewUnstaged), changes.unstaged),
                        (i18n::tr!(OverviewUntracked), changes.untracked),
                        (i18n::tr!(OverviewConflicts), changes.conflicts),
                    ];
                    let text = values
                        .into_iter()
                        .filter(|(_, count)| *count > 0)
                        .map(|(label, count)| format!("{label} {count}"))
                        .collect::<Vec<_>>()
                        .join(" · ");
                    lines.push(field(
                        i18n::tr!(Worktree),
                        if text.is_empty() {
                            i18n::tr!(CleanAlt).into()
                        } else {
                            text
                        },
                        if changes.conflicts > 0 {
                            BAD
                        } else {
                            Color::Reset
                        },
                    ));
                }
                Err(error) => lines.push(field(i18n::tr!(Worktree), safe(error), BAD)),
            }
            if snapshot.operation != Operation::Clean {
                let operation = match snapshot.operation {
                    Operation::Merge => i18n::tr!(OverviewMerging),
                    Operation::Revert | Operation::RevertSequence => i18n::tr!(OverviewReverting),
                    Operation::CherryPick | Operation::CherryPickSequence => {
                        i18n::tr!(OverviewCherryPicking)
                    }
                    Operation::Bisect => i18n::tr!(OverviewBisecting),
                    Operation::Rebase | Operation::RebaseInteractive | Operation::RebaseMerge => {
                        i18n::tr!(OverviewRebasing)
                    }
                    Operation::ApplyMailbox => i18n::tr!(OverviewApplyingMail),
                    Operation::ApplyMailboxOrRebase => i18n::tr!(OverviewMailboxOrRebase),
                    Operation::Clean => unreachable!(),
                };
                lines.push(field(i18n::tr!(OverviewOperation), operation, WARN));
            }
            if presentation == Presentation::Expanded {
                lines.push(field(
                    i18n::tr!(OverviewSnapshot),
                    local_time(&snapshot.captured_at),
                    MUTED,
                ));
            }
            if matches!(screen.overview.content, Content::Refreshing(_)) {
                lines.push(Line::styled(
                    format!(
                        "{} {}",
                        screen.overview.spinner(),
                        i18n::tr!(OverviewLoading)
                    ),
                    Style::default().fg(ACCENT),
                ));
            }
        }
        Content::Closed => {}
    }
    if let Some(path) = path {
        lines.extend(super::remotes::lines(screen, path));
        if let Some(observation) = screen.repository_check(path) {
            if presentation == Presentation::Expanded {
                lines.push(Line::default());
            }
            lines.push(Line::styled(
                i18n::tr!(OverviewCheck),
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            ));
            lines.push(field(
                i18n::tr!(CheckUpdatedAt),
                local_time(&observation.finished_at.to_rfc3339()),
                MUTED,
            ));
            let row = crate::Row {
                path: path.to_owned(),
                result: Some(observation.result.clone()),
            };
            let check = repository::detail_text(&row, repository::Scope::Check);
            lines.extend(check.lines.into_iter().skip(1));
        }
    }
    review::styled_text_view(
        frame,
        area,
        Text::from(lines),
        &mut screen.scroll,
        &title,
        &mut screen.pointer,
        crate::pointer::ScrollTarget::Overview,
    );
}
fn field(label: &str, value: impl Into<String>, color: Color) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            i18n::tf!(FieldLabel, label = label),
            Style::default().fg(MUTED),
        ),
        Span::styled(value.into(), Style::default().fg(color)),
    ])
}
