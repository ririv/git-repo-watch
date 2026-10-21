use super::*;
pub(super) fn commit_review(plan: &CommitPlan) -> String {
    let mut text = format!(
        "仓库：{}\n分支：{}\n范围：仅已暂存内容\n\n暂存文件\n",
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
) {
    styled_text_view(frame, area, Text::from(text.to_owned()), scroll, title);
}

pub(super) fn styled_text_view(
    frame: &mut Frame,
    area: Rect,
    text: Text<'static>,
    scroll: &mut usize,
    title: &str,
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
}
