//! Source and rendered preview panes share geometry for rendering and pointer focus.
use super::*;
use crate::browser::opening::{Opening, Presentation, TextMode};

pub(super) struct Pane {
    pub frame: Rect,
    pub content: Rect,
    pub mode: TextMode,
    pub heading: Option<(Rect, &'static str)>,
    pub separator: Option<Rect>,
}
pub(super) fn panes(area: Rect, session: &Session) -> Vec<Pane> {
    build(
        area,
        session.presentation(),
        session
            .open_target()
            .map(|target| target.opening(TextMode::Source)),
    )
}
pub(super) fn retained(area: Rect, snapshot: &crate::browser::panes::Snapshot) -> Vec<Pane> {
    build(
        area,
        snapshot.target.presentation,
        Some(snapshot.target.opening),
    )
}
fn build(area: Rect, presentation: Presentation, opening: Option<Opening>) -> Vec<Pane> {
    let inner = Block::bordered().inner(area);
    if !matches!(presentation, Presentation::Split(_)) || inner.width < 2 || inner.height < 2 {
        return vec![Pane {
            frame: area,
            content: inner,
            mode: presentation.active(),
            heading: None,
            separator: None,
        }];
    }
    // 较窄窗口改为上下分屏，最小窗口只显示当前焦点，仍保留分屏状态。
    let horizontal = inner.width >= 40;
    let parts = if horizontal {
        Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)]).split(area)
    } else {
        Layout::vertical([Constraint::Percentage(50), Constraint::Percentage(50)]).split(area)
    };
    [TextMode::Source, TextMode::Preview]
        .into_iter()
        .enumerate()
        .map(|(index, mode)| {
            let frame = if horizontal {
                if index == 0 {
                    Rect {
                        width: parts[0].width.saturating_add(1),
                        ..parts[0]
                    }
                } else {
                    parts[1]
                }
            } else if index == 0 {
                Rect {
                    height: parts[0].height.saturating_add(1),
                    ..parts[0]
                }
            } else {
                parts[1]
            };
            let content = Block::bordered().inner(frame);
            let heading = Rect::new(
                content.x,
                frame.y + u16::from(index == 0 || horizontal),
                content.width,
                u16::from(content.height > 0),
            );
            let body = if index == 0 || horizontal {
                Rect {
                    y: content.y.saturating_add(u16::from(content.height > 0)),
                    height: content.height.saturating_sub(1),
                    ..content
                }
            } else {
                content
            };
            Pane {
                frame,
                content: body,
                mode,
                heading: Some((
                    heading,
                    opening
                        .expect("split file capability")
                        .in_mode(mode)
                        .label(),
                )),
                separator: (index == 1 && horizontal).then_some(Rect::new(
                    frame.x,
                    inner.y,
                    1,
                    inner.height,
                )),
            }
        })
        .collect()
}

pub(super) fn decorate(frame: &mut Frame, pane: &Pane, active: bool) {
    if let Some((header, label)) = pane.heading {
        frame.render_widget(
            Paragraph::new(label).style(Style::default().fg(if active { ACCENT } else { MUTED })),
            header,
        );
    }
    if let Some(separator) = pane.separator {
        frame.render_widget(
            Block::default()
                .borders(Borders::LEFT)
                .border_style(Style::default().fg(MUTED)),
            separator,
        );
    }
}
