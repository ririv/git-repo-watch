//! Changed files retain the shared trailing status markers and precise hit regions.
use super::*;
use crate::search::highlight::{self, Emphasis};
use engine::local::history::Kind;
pub(super) fn render(
    frame: &mut Frame,
    area: Rect,
    commit: &mut Session,
    icons: crate::file_icons::Display,
    hits: &mut crate::pointer::Map,
) {
    let rows = commit.rows();
    let count = rows.len();
    let mut state = commit.files;
    let title = format!("{} · {}", i18n::tr!(HistoryChanges), rows.len());
    let border = block(title);
    let inner = border.inner(area);
    frame.render_widget(border, area);
    if rows.is_empty() {
        frame.render_widget(
            Paragraph::new(commit.detail.empty_message(&commit.query))
                .wrap(Wrap { trim: false })
                .style(Style::default().fg(MUTED)),
            inner,
        );
    } else {
        let visible = list_window::visible(inner, count, &mut state);
        let selection = row_selection(state.selected(), count);
        list_window::render(frame, inner, visible.clone(), |index| {
            let change = rows[index];
            let color = match change.kind {
                Kind::Added => GOOD,
                Kind::Deleted => BAD,
                Kind::Modified | Kind::Typechange => WARN,
                Kind::Renamed | Kind::Copied => ACCENT,
            };
            let path = highlight::line(
                crate::view::files::path_line(&change.location, &change.location.display()),
                &commit.query,
                Emphasis::at(index, selection),
            );
            let mut spans = icons.file(change.location.name());
            spans.extend(path.spans);
            let path = Line::from(spans);
            ListItem::new(crate::view::files::marked_line(
                &path,
                change.kind.symbol(),
                color,
                inner.width as usize,
            ))
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
