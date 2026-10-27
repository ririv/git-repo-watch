//! A reserved single row links transient notices to persistent tasks.
mod status;
use super::*;
#[derive(Debug, Clone, Copy)]
pub(super) enum Preview {
    Hidden,
    Reserved,
}
impl Preview {
    pub fn fit(width: u16, budget: u16) -> Self {
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
        let exit_notice = screen.exit_sequence.notice();
        if exit_notice.is_none() {
            screen.tasks.footer = area;
        }
        let text = status::line(screen, area.width.into());
        frame.render_widget(Paragraph::new(text), area);
    }
}
