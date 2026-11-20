//! Responsive list titles with clickable fields.
use super::*;
use crate::pointer;

const SEPARATOR: &str = " · ";

pub(super) struct Field {
    text: Line<'static>,
    action: pointer::Action,
}
impl Field {
    pub fn new(label: &'static str, value: &'static str, action: pointer::Action) -> Self {
        Self {
            text: Line::from(vec![
                Span::styled(
                    i18n::tf!(FieldLabel, label = label),
                    Style::default().fg(MUTED),
                ),
                Span::styled(value, Style::default().fg(ACCENT)),
            ]),
            action,
        }
    }
}
pub(super) fn line(fields: &[Field]) -> Line<'static> {
    let mut spans = Vec::new();
    for (index, field) in fields.iter().enumerate() {
        if index > 0 {
            spans.push(Span::styled(SEPARATOR, Style::default().fg(MUTED)));
        }
        spans.extend(field.text.spans.iter().cloned());
    }
    Line::from(spans)
}
fn buttons(fields: &[Field], map: &mut pointer::Map, mut x: u16, y: u16) {
    for field in fields {
        let width = field.text.width() as u16;
        map.button(Rect::new(x, y, width, 1), field.action);
        x += width + Line::from(SEPARATOR).width() as u16;
    }
}
pub(super) fn render(
    frame: &mut Frame,
    title: &str,
    padding: ratatui::widgets::Padding,
    fields: &[Field],
    area: Rect,
    pointer: &mut pointer::Map,
) -> Rect {
    // 极矮窗口先给列表留空间。
    if area.height < 3 {
        return area;
    }
    let mut options = line(fields);
    let width = options.width() as u16;
    let border = block(title).padding(padding);
    let content = border.inner(area);
    // 标题与选项之间留一格边框，选项两侧各留一格空白。
    if Line::from(title).width() + usize::from(width) + 3
        <= usize::from(area.width.saturating_sub(2))
    {
        buttons(fields, pointer, area.right() - width - 2, area.y);
        options.spans.insert(0, Span::raw(" "));
        options.spans.push(Span::raw(" "));
        frame.render_widget(border.title(options.right_aligned()), area);
        return content;
    }
    frame.render_widget(border, area);
    if content.height >= 3 && width <= content.width {
        buttons(fields, pointer, content.x, content.y);
        let parts = Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).split(content);
        frame.render_widget(Paragraph::new(options), parts[0]);
        parts[1]
    } else {
        content
    }
}
