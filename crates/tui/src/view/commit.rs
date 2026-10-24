//! Staged changes above a message document, with horizontal actions in the footer.
use super::*;
use crate::commit_form::Draft;
pub(crate) fn areas(area: Rect) -> (Rect, Rect) {
    let parts =
        Layout::vertical([Constraint::Percentage(60), Constraint::Percentage(40)]).split(area);
    (parts[1], parts[0])
}
pub(crate) fn options(
    area: Rect,
    draft: &Draft,
    ai: bool,
) -> Vec<(Rect, crate::commit_form::Action)> {
    let actions = draft.actions(ai);
    let widths = actions
        .iter()
        .map(|action| unicode_width::UnicodeWidthStr::width(action.label()) as u16 + 2)
        .collect::<Vec<_>>();
    let padding = u16::from(area.width > 1);
    let mut start = 0;
    if let Some(selected) = draft
        .selected()
        .and_then(|selected| actions.iter().position(|action| *action == selected))
    {
        // 窄窗口仅平移操作项，始终让当前选择可见，不额外增加一行。
        while start < selected
            && widths[start..=selected].iter().sum::<u16>() + 2 * (selected - start) as u16
                > area.width.saturating_sub(padding)
        {
            start += 1;
        }
    }
    let mut x = area.x.saturating_add(padding).min(area.right());
    actions
        .iter()
        .enumerate()
        .map(|(index, &action)| {
            if index < start {
                return (Rect::new(area.x, area.y, 0, area.height.min(1)), action);
            }
            let width = widths[index].min(area.right().saturating_sub(x));
            let button = Rect::new(x, area.y, width, area.height.min(1));
            x = x.saturating_add(width + 2).min(area.right());
            (button, action)
        })
        .collect()
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
    text_view(
        frame,
        review,
        &commit_review(plan),
        scroll,
        i18n::tr!(CommitPreview),
        hits,
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
