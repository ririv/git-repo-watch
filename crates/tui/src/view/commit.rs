//! Staged changes above a message document, with horizontal actions in the footer.
use super::*;
use crate::commit_form::Draft;
use engine::local::review::{CommitFile, FileDiff, LineCounts};

pub(crate) fn areas(area: Rect) -> (Rect, Rect) {
    let parts =
        Layout::vertical([Constraint::Percentage(60), Constraint::Percentage(40)]).split(area);
    (parts[1], parts[0])
}
pub(super) fn action_row(draft: &Draft, ai: bool) -> input::actions::Row {
    use input::actions::{Item, Row, Spacing, State, Target};

    let selected = draft.selected();
    let items = draft
        .actions(ai)
        .iter()
        .copied()
        .map(|action| Item {
            label: action.label(),
            state: State::new(true, selected == Some(action)),
            target: Target::Button(crate::pointer::Action::CommitFooter(action)),
        })
        .collect();
    Row::new(items, Spacing::Indented)
}

#[cfg(test)]
pub(crate) fn options(
    area: Rect,
    draft: &Draft,
    ai: bool,
) -> Vec<(Rect, crate::commit_form::Action)> {
    action_row(draft, ai)
        .cells(area)
        .into_iter()
        .zip(draft.actions(ai).iter().copied())
        .map(|((area, _), action)| (area, action))
        .collect()
}

fn line_count_spans(LineCounts { added, removed }: LineCounts) -> [Span<'static>; 3] {
    [
        Span::styled(
            format!("+{added}"),
            Style::default().fg(if added == 0 { MUTED } else { GOOD }),
        ),
        Span::raw(" "),
        Span::styled(
            format!("-{removed}"),
            Style::default().fg(if removed == 0 { MUTED } else { BAD }),
        ),
    ]
}

fn file_rows(file: &CommitFile, width: usize) -> Vec<Line<'static>> {
    let name = Line::raw(format!("• {}", safe(&file.location.display())));
    let statistic = match file.diff {
        FileDiff::Text(counts) => Line::from(line_count_spans(counts).to_vec()),
        FileDiff::Binary => {
            Line::styled(i18n::tr!(CommitPreviewBinary), Style::default().fg(MUTED))
        }
    };
    let stats_width = statistic.width();
    let path_width = width.saturating_sub(stats_width + 2);
    if path_width < 4 {
        return vec![name, statistic.right_aligned()];
    }
    let mut rows = if name.width() <= path_width {
        vec![name]
    } else {
        crate::links::Line::from(name)
            .wrap_pinned(path_width, 2)
            .into_iter()
            .map(|row| row.text)
            .collect()
    };
    let first = &mut rows[0];
    first
        .spans
        .push(Span::raw(" ".repeat(width - first.width() - stats_width)));
    first.spans.extend(statistic.spans);
    rows
}

fn commit_review(plan: &CommitPlan, width: usize) -> Text<'static> {
    let mut text = Text::from(i18n::tf!(
        RepositoryBranchScopeStagedContentsOnlyStaged,
        safe(&plan.path.to_string_lossy()),
        safe(plan.branch())
    ));
    text.lines
        .extend(plan.files.iter().flat_map(|file| file_rows(file, width)));
    let mut summary = Line::raw(format!(
        "{} · ",
        i18n::tf!(CommitPreviewFileCount, plan.files.len())
    ));
    summary.spans.extend(line_count_spans(plan.totals));
    text.lines.extend([Line::default(), summary]);
    text
}

pub(super) fn render(
    frame: &mut Frame,
    area: Rect,
    plan: &CommitPlan,
    draft: &mut Draft,
    scroll: &mut usize,
    hits: &mut crate::pointer::Map,
    plain: bool,
) {
    let (message, review) = areas(area);
    review::styled_text_view(
        frame,
        review,
        commit_review(plan, usize::from(review.width.saturating_sub(2))),
        scroll,
        i18n::tr!(CommitPreview),
        hits,
        crate::pointer::ScrollTarget::Text,
    );
    let focused = draft.focused() && plain;
    let title = if draft.submitting() {
        format!(
            "{} · {}",
            i18n::tr!(CommitMessageAlt).trim(),
            i18n::tr!(CommitSubmitting)
        )
    } else if draft.generating() {
        format!(
            "{} · {}",
            i18n::tr!(CommitMessageAlt).trim(),
            i18n::tr!(AgentGenerating)
        )
    } else {
        i18n::tr!(CommitMessageAlt).trim().into()
    };
    let panel = block(title).padding(ratatui::widgets::Padding::horizontal(1));
    let inner = panel.inner(message);
    frame.render_widget(panel, message);
    if focused {
        highlight_border(frame, message);
    }
    let placeholder = if draft.generating() {
        i18n::tr!(AgentGenerating)
    } else {
        i18n::tr!(DescribeThisCommit)
    };
    let editor = draft.display();
    let count = super::text_editor::render(frame, inner, editor, placeholder, focused);
    crate::scrollbar::vertical(frame, message, inner, count, editor.scroll);
    hits.vertical(
        message,
        inner,
        count.saturating_sub(inner.height.into()),
        editor.scroll,
        crate::pointer::ScrollTarget::CommitMessage,
    );
}
