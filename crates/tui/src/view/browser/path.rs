//! A file path occupies a stable row independently from application breadcrumbs.
use super::*;
use unicode_width::UnicodeWidthStr;

struct Part {
    label: String,
    role: Role,
}
enum Role {
    Ancestor(usize),
    Current,
    Omitted,
}
pub(super) fn areas(area: Rect) -> (Rect, Rect) {
    let height = u16::from(area.height >= 4);
    (
        Rect { height, ..area },
        Rect {
            y: area.y + height,
            height: area.height.saturating_sub(height),
            ..area
        },
    )
}
fn width(parts: &[Part]) -> usize {
    parts.iter().map(|part| part.label.width()).sum::<usize>() + 3 * parts.len().saturating_sub(1)
}
pub(super) fn render(
    frame: &mut Frame,
    area: Rect,
    session: &Session,
    hits: &mut crate::pointer::Map,
) {
    if area.is_empty() {
        return;
    }
    let location = session.location();
    let path = location.path();
    let count = path.components().count();
    let mut parts = vec![Part {
        label: name(&session.root),
        role: if count > 0 {
            Role::Ancestor(0)
        } else {
            Role::Current
        },
    }];
    parts.extend(
        path.components()
            .enumerate()
            .map(|(index, component)| Part {
                label: safe(&component.as_os_str().to_string_lossy()),
                role: if index + 1 < count {
                    Role::Ancestor(index + 1)
                } else {
                    Role::Current
                },
            }),
    );
    let available = usize::from(area.width.saturating_sub(2));
    let mut omitted = false;
    while width(&parts) + usize::from(omitted) * 4 > available && parts.len() > 3 {
        parts.remove(1);
        omitted = true;
    }
    if omitted {
        parts.insert(
            1,
            Part {
                label: "…".into(),
                role: Role::Omitted,
            },
        );
    }
    while 4 * parts.len() - 3 > available && parts.len() > 1 {
        parts.remove(0);
    }
    while width(&parts) > available {
        let part = parts
            .iter_mut()
            .max_by_key(|part| part.label.width())
            .expect("path has a root");
        part.label =
            super::super::breadcrumbs::shorten(&part.label, part.label.width().saturating_sub(1));
    }
    let mut x = area.x + 1;
    for (index, part) in parts.into_iter().enumerate() {
        if index > 0 {
            frame.render_widget(
                Paragraph::new(" › ").style(Style::default().fg(MUTED)),
                Rect::new(x, area.y, 3, 1),
            );
            x += 3;
        }
        let cell = Rect::new(x, area.y, part.label.width() as u16, 1).intersection(area);
        let style = match part.role {
            Role::Ancestor(depth) => {
                hits.button(cell, crate::pointer::Action::BrowserDirectory(depth));
                Style::default().fg(ACCENT).underlined()
            }
            Role::Current => Style::default().fg(Color::White).bold(),
            Role::Omitted => Style::default().fg(MUTED),
        };
        frame.render_widget(Paragraph::new(part.label).style(style), cell);
        x += cell.width;
    }
}
