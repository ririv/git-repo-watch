use super::*;
pub(super) fn commit_review(plan: &CommitPlan) -> String {
    let mut text = i18n::tf!(
        RepositoryBranchScopeStagedContentsOnlyStaged,
        safe(&plan.path.to_string_lossy()),
        safe(plan.branch())
    );
    for file in &plan.files {
        text.push_str(&format!("• {file:?}\n"))
    }
    text.push_str(&format!("\n{}", plan.stat));
    text
}
pub(super) fn text_view(
    frame: &mut Frame,
    area: Rect,
    text: &str,
    scroll: &mut usize,
    title: &str,
    hits: &mut crate::pointer::Map,
) {
    styled_text_view(
        frame,
        area,
        Text::from(text.to_owned()),
        scroll,
        title,
        hits,
        crate::pointer::ScrollTarget::Text,
    );
}

pub(super) fn styled_text_view(
    frame: &mut Frame,
    area: Rect,
    text: Text<'static>,
    scroll: &mut usize,
    title: &str,
    hits: &mut crate::pointer::Map,
    target: crate::pointer::ScrollTarget,
) {
    let block = block(title.to_owned());
    let inner = block.inner(area);
    let paragraph = Paragraph::new(text).wrap(Wrap { trim: false });
    let lines = paragraph.line_count(inner.width);
    *scroll = (*scroll).min(lines.saturating_sub(inner.height as usize));
    frame.render_widget(
        paragraph
            .scroll(((*scroll).min(u16::MAX as usize) as u16, 0))
            .block(block),
        area,
    );
    crate::scrollbar::vertical(frame, area, inner, lines, *scroll);
    hits.vertical(
        area,
        inner,
        lines.saturating_sub(inner.height as usize),
        *scroll,
        target,
    );
}
