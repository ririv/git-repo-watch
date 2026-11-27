//! Changed files retain the shared trailing status markers and precise hit regions.
use super::*;
use crate::search::highlight::{self, Emphasis};
use crate::{horizontal::Pane, view::list_window};
pub(super) fn render(
    frame: &mut Frame,
    area: Rect,
    commit: &mut Session,
    icons: crate::file_icons::Display,
    hits: &mut crate::pointer::Map,
) {
    let rows: Vec<_> = commit
        .indices()
        .into_iter()
        .map(|index| commit.detail.change(index))
        .collect();
    let count = rows.len();
    let mut state = commit.files;
    let title = format!("{} · {}", i18n::tr!(HistoryChanges), rows.len());
    let border = block(title);
    let inner = border.inner(area);
    frame.render_widget(border, area);
    if rows.is_empty() {
        commit.file_paths.constrain(0);
        frame.render_widget(
            Paragraph::new(commit.detail.empty_message(&commit.query))
                .wrap(Wrap { trim: false })
                .style(Style::default().fg(MUTED)),
            inner,
        );
    } else {
        let visible = list_window::visible(inner, count, &mut state);
        let selection = row_selection(state.selected(), count);
        let labels: Vec<_> = rows
            .iter()
            .map(|change| crate::view::files::history_label(change, icons))
            .collect();
        let widest = labels.iter().map(Line::width).max().unwrap_or(0);
        let paths = crate::horizontal::Columns::new(inner, 0, 2).scroll(
            frame,
            area,
            hits,
            Pane::Inspection,
            &mut commit.file_paths,
            widest,
        );
        list_window::render(frame, inner, visible.clone(), |index| {
            let label = highlight::line(
                labels[index].clone(),
                &commit.query,
                Emphasis::at(index, selection),
            );
            let status = crate::view::files::history_status(rows[index].kind);
            ListItem::new(paths.line(&label, Some(status)))
                .style(row_style(selection == Some(index)))
        });
        commit.files = state;
        crate::scrollbar::vertical(frame, area, inner, count, commit.files.offset());
        hits.sequential_list(
            area,
            inner,
            count,
            visible,
            crate::pointer::Target::InspectionFiles,
        );
    }
}
