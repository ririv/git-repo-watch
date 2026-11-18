//! A list of single-line items keeps global selection and builds only visible rows.
use super::*;
use std::ops::Range;

pub(super) fn visible(area: Rect, count: usize, state: &mut ListState) -> Range<usize> {
    if area.is_empty() {
        return 0..0;
    }
    if count == 0 {
        state.select(None);
        return 0..0;
    }
    let height = usize::from(area.height);
    let mut start = state.offset().min(count - 1);
    if let Some(selected) = state.selected() {
        let selected = selected.min(count - 1);
        state.select(Some(selected));
        start = start.clamp(selected.saturating_sub(height - 1), selected);
    }
    *state.offset_mut() = start;
    start..start.saturating_add(height).min(count)
}

pub(super) fn render(
    frame: &mut Frame,
    area: Rect,
    rows: Range<usize>,
    item: impl FnMut(usize) -> ListItem<'static>,
) {
    frame.render_widget(List::new(rows.map(item)), area);
}
