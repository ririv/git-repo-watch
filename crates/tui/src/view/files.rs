//! Shared file list for full-size reviews and embedded repository panes.
use super::*;
pub(super) fn render(frame: &mut Frame, area: Rect, s: &FileSession, title: &str) {
    let frame_block = block(title.to_owned());
    let inner = frame_block.inner(area);
    frame.render_widget(frame_block, area);
    let parts = Layout::vertical([Constraint::Length(2), Constraint::Min(1)]).split(inner);
    let rows = s.rows();
    frame.render_widget(
        Paragraph::new(i18n::tf!(
            FilesSelected,
            if s.area == FileArea::Index {
                i18n::tr!(Index)
            } else {
                i18n::tr!(Worktree)
            },
            rows.len(),
            s.selected.len()
        )),
        parts[0],
    );
    let items = rows
        .iter()
        .map(|c| {
            ListItem::new(format!(
                "[{}] {} {}",
                if s.selected.contains(&s.change_index(c)) {
                    "✓"
                } else {
                    " "
                },
                c.kind,
                safe(&c.location.display())
            ))
        })
        .collect::<Vec<_>>();
    let mut state = ListState::default().with_selected(if items.is_empty() {
        None
    } else {
        Some(s.cursor)
    });
    frame.render_stateful_widget(
        List::new(if items.is_empty() {
            vec![ListItem::new(i18n::tr!(NoChangesInThisArea))]
        } else {
            items
        })
        .highlight_style(selected_style())
        .highlight_symbol("› "),
        parts[1],
        &mut state,
    );
}
