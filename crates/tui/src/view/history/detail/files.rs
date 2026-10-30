//! Changed files retain the shared trailing status markers and precise hit regions.
use super::*;
use crate::search::highlight::{self, Emphasis};
use engine::local::{files::Location, history::Kind};
pub(super) fn render(
    frame: &mut Frame,
    area: Rect,
    commit: &mut Commit,
    hits: &mut crate::pointer::Map,
) {
    let rows = commit.rows();
    let count = rows.len();
    let selection = row_selection(commit.files.selected(), count);
    let title = format!("{} · {}", i18n::tr!(HistoryChanges), rows.len());
    let border = block(title);
    let inner = border.inner(area);
    frame.render_widget(border, area);
    if rows.is_empty() {
        frame.render_widget(
            Paragraph::new(if commit.query.is_empty() {
                i18n::tr!(HistoryNoChanges)
            } else {
                i18n::tr!(HistoryNoFilesMatch)
            })
            .wrap(Wrap { trim: false })
            .style(Style::default().fg(MUTED)),
            inner,
        );
    } else {
        let items: Vec<_> = rows
            .iter()
            .enumerate()
            .map(|(index, change)| {
                let path = match &change.location {
                    Location::Path(path) => path.display().to_string(),
                    Location::Rename { before, after } | Location::Copy { before, after } => {
                        format!("{} → {}", before.display(), after.display())
                    }
                };
                let color = match change.kind {
                    Kind::Added => GOOD,
                    Kind::Deleted => BAD,
                    Kind::Modified | Kind::Typechange => WARN,
                    Kind::Renamed | Kind::Copied => ACCENT,
                };
                let path = highlight::line(
                    Line::raw(safe(&path)),
                    &commit.query,
                    Emphasis::at(index, selection),
                );
                ListItem::new(crate::view::files::marked_line(
                    &path,
                    change.kind.symbol(),
                    color,
                    inner.width as usize,
                ))
                .style(row_style(selection == Some(index)))
            })
            .collect();
        frame.render_stateful_widget(List::new(items), inner, &mut commit.files);
        crate::scrollbar::vertical(frame, area, inner, count, commit.files.offset());
        hits.list(
            area,
            inner,
            &(0..count).collect::<Vec<_>>(),
            commit.files.offset(),
            crate::pointer::Target::HistoryFiles,
        );
    }
}
