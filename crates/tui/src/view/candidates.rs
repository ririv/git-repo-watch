//! Titleless candidates share a border with the command input when space permits.
use super::*;

pub(super) fn panel(frame: &mut Frame, screen: &Screen, body: Rect, count: usize) -> (Rect, Rect) {
    let input = screen.input_area.filter(|input| {
        input.height >= 3
            && input.width >= 2
            && input.x == body.x
            && input.width == body.width
            && input.y == body.bottom()
            && body.height >= 2
    });
    let available = body.height.saturating_sub(u16::from(body.height >= 9));
    let height = (count.clamp(1, 8) as u16 + if input.is_some() { 1 } else { 2 }).min(available);
    let area = Rect::new(
        body.x,
        body.bottom().saturating_sub(height),
        body.width,
        height,
    );
    let border = match input {
        Some(input) => {
            // 菜单不画底边，与输入框共用一条分隔线，避免双边框和标题重复。
            input::connect_menu(frame, screen, input);
            Block::default()
                .borders(Borders::TOP | Borders::LEFT | Borders::RIGHT)
                .border_type(ratatui::widgets::BorderType::Rounded)
                .border_style(Style::default().fg(input::active_accent(screen)))
        }
        None if height >= 3 => Block::bordered().border_style(Style::default().fg(BORDER)),
        None => Block::default(),
    };
    let inner = border.inner(area);
    frame.render_widget(Clear, area);
    frame.render_widget(border, area);
    (area, inner)
}
