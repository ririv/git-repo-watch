//! Responsive file groups and text or raster comparison share one screen.
use super::*;
use crate::{
    changes::{Focus, Preview, Session},
    workspace::Arrangement,
};

pub(crate) fn areas(area: Rect, session: &Session) -> Vec<(Rect, Focus)> {
    if session.arrangement == Arrangement::Maximized || area.width < 60 || area.height < 8 {
        return vec![(area, session.focus)];
    }
    let parts = if area.width >= 110 {
        Layout::horizontal([
            Constraint::Length((area.width / 3).clamp(28, 42)),
            Constraint::Min(1),
        ])
        .split(area)
    } else {
        Layout::vertical([
            Constraint::Length((area.height / 3).clamp(5, 10)),
            Constraint::Min(1),
        ])
        .split(area)
    };
    vec![(parts[0], Focus::Files), (parts[1], Focus::Preview)]
}
pub(super) fn render(
    frame: &mut Frame,
    area: Rect,
    session: &mut Session,
    preference: crate::diff::Preference,
    file_layout: crate::files::Layout,
    graphics: &mut crate::graphics::Display,
    hits: &mut crate::pointer::Map,
) {
    let symbol = session.symbol();
    for (area, focus) in areas(area, session) {
        match focus {
            Focus::Files => {
                let title = name(&session.files.snapshot.path);
                files::render(frame, area, &mut session.files, &title, file_layout, hits);
            }
            Focus::Preview => match &mut session.preview {
                Preview::Ready {
                    content, scroll, ..
                } => document::render(frame, area, content, scroll, preference, graphics, hits),
                state => {
                    let text = match state {
                        Preview::Empty => i18n::tr!(ChangesSelectFile).to_owned(),
                        Preview::Loading(_) => format!("{} {}", symbol, i18n::tr!(ReadFileDiff)),
                        Preview::Failed { error, .. } => multiline(error),
                        Preview::Ready { .. } => unreachable!(),
                    };
                    frame.render_widget(
                        Paragraph::new(text)
                            .wrap(Wrap { trim: false })
                            .block(block(i18n::tr!(FileDiff))),
                        area,
                    );
                }
            },
        }
        if focus == session.focus {
            highlight_border(frame, area);
        }
    }
}
