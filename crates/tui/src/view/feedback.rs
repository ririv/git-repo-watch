//! A reserved single row links transient notices to persistent tasks.
use super::*;
#[derive(Debug, Clone, Copy)]
pub(super) enum Preview {
    Hidden,
    Reserved,
}
impl Preview {
    pub fn fit(_text: &str, width: u16, budget: u16) -> Self {
        if width > FOOTER_INSET * 2 && budget > 0 {
            Self::Reserved
        } else {
            Self::Hidden
        }
    }
    pub fn height(self) -> u16 {
        match self {
            Self::Hidden => 0,
            Self::Reserved => 1,
        }
    }
    pub fn render(self, frame: &mut Frame, screen: &mut Screen, area: Rect) {
        let area = footer_text_area(area);
        screen.tasks.footer = Rect::default();
        if matches!(self, Self::Hidden) {
            return;
        }
        screen.tasks.footer = area;
        let text = if screen.tasks.error.is_none()
            && !screen
                .tasks
                .records
                .iter()
                .any(|record| record.active() || record.pending())
            && !screen.message.is_empty()
        {
            screen.message.lines().next().unwrap_or("").into()
        } else {
            screen.tasks.footer_text()
        };
        let color = if screen
            .tasks
            .records
            .iter()
            .any(engine::tasks::Record::pending)
            || screen.tasks.error.is_some()
        {
            WARN
        } else {
            MUTED
        };
        frame.render_widget(
            Paragraph::new(truncate(&text, area.width.into())).style(Style::default().fg(color)),
            area,
        );
    }
}
