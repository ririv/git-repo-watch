//! Keep application metadata fixed above the independently scrolling Git patch.
use super::*;

pub(super) fn render(
    frame: &mut Frame,
    area: Rect,
    content: &crate::diff::View,
    mode: DiffLayout,
    hits: &mut crate::pointer::Map,
) -> Rect {
    let mut details = Vec::new();
    if let Some(information) = &content.information {
        details.push(Line::from(Span::styled(
            information.title.clone(),
            Style::default().fg(Color::White),
        )));
        for syntax in &information.syntax {
            details.push(Line::from(vec![
                Span::styled(
                    format!("{}: ", i18n::tr!(SyntaxLabel)),
                    Style::default().fg(MUTED),
                ),
                Span::raw(syntax.clone()),
            ]));
        }
    }
    let details = hits.links.text(Text::from(details));
    let lines = details
        .lines
        .iter()
        .flat_map(|line| line.wrap_pinned(area.width as usize, 0))
        .collect::<Vec<_>>();
    // 信息区随宽度换行；高度不足时优先为补丁保留一行。
    let height = (lines.len() + 1).min(area.height.saturating_sub(1) as usize) as u16;
    if height == 0 {
        return area;
    }
    for (index, line) in lines
        .iter()
        .take(height.saturating_sub(1) as usize)
        .enumerate()
    {
        hits.links.line(
            frame,
            Rect::new(area.x, area.y + index as u16, area.width, 1),
            line,
            0,
            0,
        );
    }
    comparison(
        frame,
        Rect::new(area.x, area.y + height - 1, area.width, 1),
        content.labels(),
        mode,
    );
    Rect::new(area.x, area.y + height, area.width, area.height - height)
}

fn comparison(frame: &mut Frame, area: Rect, labels: (String, String), mode: DiffLayout) {
    let (before, after) = labels;
    let style = Style::default().fg(ACCENT).bold();
    frame.render_widget(
        Line::styled("─".repeat(area.width as usize), separator_style()),
        area,
    );
    // 来源标签嵌入分隔线；双栏的连接点与正文列边界保持一致。
    frame.buffer_mut()[(area.x - 1, area.y)].set_symbol("├");
    frame.buffer_mut()[(area.right(), area.y)].set_symbol("┤");
    match mode {
        DiffLayout::Unified => frame.render_widget(
            Line::from(Span::styled(format!(" {before} → {after} "), style)),
            area,
        ),
        DiffLayout::Split => {
            let left = area.width.saturating_sub(1) / 2;
            let right = area.width.saturating_sub(1) - left;
            frame.render_widget(
                Line::from(Span::styled(format!(" {before} "), style)),
                Rect::new(area.x, area.y, left, 1),
            );
            frame.render_widget(
                Line::from(Span::styled(format!(" {after} "), style)),
                Rect::new(area.x + left + 1, area.y, right, 1),
            );
            frame.render_widget(
                Line::styled("┬", separator_style()),
                Rect::new(area.x + left, area.y, 1, 1),
            );
        }
    }
}
