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
    preference: crate::diff::Options,
    files_options: crate::files::Options,
    graphics: &mut crate::graphics::Display,
    hits: &mut crate::pointer::Map,
) {
    let symbol = session.symbol();
    let loading_visible = session.feedback.visible();
    for (area, focus) in areas(area, session) {
        let mut outline = true;
        match focus {
            Focus::Files => {
                files::render(frame, area, &mut session.files, "", files_options, hits);
            }
            Focus::Preview => {
                let mut retained_hits = crate::pointer::Map::default();
                let (preview, hits) = match session.feedback.previous_mut() {
                    Some(previous) => (previous.as_mut(), &mut retained_hits),
                    None => (&mut session.preview, &mut *hits),
                };
                match preview {
                    Preview::Ready {
                        content,
                        scroll,
                        editing,
                        ..
                    } => {
                        if let crate::document::View::Text(view) = content.as_mut() {
                            let selection = match (session.focus, editing) {
                                (Focus::Preview, crate::changes::edit::State::Ready(editing)) => {
                                    Some((editing.selected(), editing.cursor(), editing.label()))
                                }
                                _ => None,
                            };
                            view.select(selection);
                        }
                        document::render(frame, area, content, scroll, preference, graphics, hits);
                        if let crate::document::View::Conflict(view) = content.as_ref() {
                            outline = false;
                            if session.focus == Focus::Preview
                                && session.files.recovery.selected().is_none()
                            {
                                super::conflict::highlight(frame, area, view.mode, view.focus());
                            }
                        }
                    }
                    state => {
                        let text = match state {
                            Preview::Empty => i18n::tr!(ChangesSelectFile).to_owned(),
                            Preview::Directory { path, count } => format!(
                                "{}/\n{} · {count}",
                                safe(&path.display().to_string()),
                                i18n::tr!(HistoryChanges)
                            ),
                            Preview::Loading(_) if loading_visible => {
                                format!("{} {}", symbol, i18n::tr!(ReadFileDiff))
                            }
                            Preview::Loading(_) => String::new(),
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
                }
            }
        }
        if outline && focus == session.focus && session.files.recovery.selected().is_none() {
            highlight_border(frame, area);
        }
        if focus == Focus::Files && area.width >= 4 && area.height >= 1 {
            groups(
                frame,
                Rect::new(area.x + 1, area.y, area.width - 2, 1),
                session,
                hits,
            );
        }
    }
}

fn groups(frame: &mut Frame, area: Rect, session: &Session, hits: &mut crate::pointer::Map) {
    use crate::files::Group;
    let cells = Layout::horizontal([Constraint::Fill(1), Constraint::Fill(1)]).split(area);
    let counts = session
        .files
        .snapshot
        .changes
        .iter()
        .fold([0usize; 2], |mut counts, change| {
            counts[Group::of(change.area).index()] += 1;
            counts
        });
    for (group, cell) in [Group::Unstaged, Group::Staged]
        .into_iter()
        .zip(cells.iter())
    {
        let style = if group == session.active_group() {
            Style::default()
                .fg(ACCENT)
                .bg(crate::appearance::SELECTION)
                .bold()
        } else {
            Style::default().fg(MUTED)
        };
        let label = format!("{} · {}", group.label(), counts[group.index()]);
        frame.render_widget(
            Paragraph::new(label)
                .alignment(ratatui::layout::Alignment::Center)
                .style(style),
            *cell,
        );
        hits.button(*cell, crate::pointer::Action::ChangesGroup(group));
    }
}
