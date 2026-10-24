//! Wrapped result previews and a full reader share the same sanitized text.
use super::*;
use crate::feedback::{Feedback, Reader};

#[derive(Debug, Clone, Copy)]
pub(super) enum Preview {
    Hidden,
    Reserved,
    Complete(u16),
    Clipped { rows: u16 },
}
impl Preview {
    pub fn fit(text: &str, width: u16, budget: u16) -> Self {
        let width = width.saturating_sub(FOOTER_INSET * 2);
        if width == 0 || budget == 0 {
            return Self::Hidden;
        }
        let text = multiline(text);
        if text.is_empty() {
            return Self::Reserved;
        }
        let lines = Paragraph::new(text)
            .wrap(Wrap { trim: false })
            .line_count(width);
        if lines <= usize::from(budget) {
            Self::Complete(lines as u16)
        } else {
            Self::Clipped { rows: budget - 1 }
        }
    }
    pub fn height(self) -> u16 {
        match self {
            Self::Hidden => 0,
            Self::Reserved => 1,
            Self::Complete(rows) => rows,
            Self::Clipped { rows } => rows + 1,
        }
    }
    pub fn render(self, frame: &mut Frame, screen: &mut Screen, area: Rect) {
        let area = footer_text_area(area);
        let (rows, expansion) = match self {
            Self::Hidden => return,
            Self::Reserved => {
                frame.render_widget(Clear, area);
                return;
            }
            Self::Complete(rows) => (rows, None),
            Self::Clipped { rows } => (rows, Some(Rect::new(area.x, area.y + rows, area.width, 1))),
        };
        let text = input::status_text(screen, screen.body_area.expect("body before footer"));
        frame.render_widget(
            Paragraph::new(multiline(&text))
                .wrap(Wrap { trim: false })
                .style(Style::default().fg(WARN)),
            Rect::new(area.x, area.y, area.width, rows),
        );
        if let Some(expansion) = expansion {
            frame.render_widget(
                Paragraph::new(truncate(
                    i18n::tr!(ExpandOperationResult),
                    expansion.width.into(),
                ))
                .style(Style::default().fg(ACCENT)),
                expansion,
            );
        }
        screen.feedback = Feedback::Inline { expansion };
    }
}
pub(super) fn reader(frame: &mut Frame, area: Rect, reader: &mut Reader) {
    let panel = block(i18n::tr!(OperationResult));
    let inner = panel.inner(area);
    let hint_height = u16::from(!inner.is_empty());
    let content = Rect::new(
        inner.x,
        inner.y,
        inner.width,
        inner.height.saturating_sub(hint_height),
    );
    let back = Rect::new(inner.x, content.bottom(), inner.width, hint_height);
    let paragraph = Paragraph::new(multiline(&reader.text)).wrap(Wrap { trim: false });
    let lines = paragraph.line_count(content.width);
    reader.maximum = lines.saturating_sub(usize::from(content.height));
    reader.scroll = reader.scroll.min(reader.maximum);
    reader.content = content;
    reader.back = back;
    frame.render_widget(Clear, area);
    frame.render_widget(panel, area);
    frame.render_widget(paragraph.scroll((reader.scroll as u16, 0)), content);
    frame.render_widget(
        Paragraph::new(truncate(
            i18n::tr!(OperationResultReaderHint),
            back.width.into(),
        ))
        .style(Style::default().fg(MUTED)),
        back,
    );
    crate::scrollbar::vertical(frame, area, content, lines, reader.scroll);
}
