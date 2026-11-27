//! Single-line lists share selection, horizontal clipping, and scrollbar hit geometry.
//!
//! Non-wrapping rows use the shared horizontal columns and viewports.
use super::*;
use crate::{
    horizontal::{self, Columns, Pane},
    pointer::Map,
};
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

pub(super) struct Scrolling<'a> {
    pub panel: Rect,
    pub inner: Rect,
    pub pane: Pane,
    pub viewport: &'a mut horizontal::Viewport,
}

impl Scrolling<'_> {
    pub fn render(
        self,
        frame: &mut Frame,
        hits: &mut Map,
        state: &mut ListState,
        rows: &[Line<'static>],
        empty: Option<&str>,
    ) {
        let widest = rows.iter().map(Line::width).max().unwrap_or(0);
        let columns = Columns::new(self.inner, 0, 0).scroll(
            frame,
            self.panel,
            hits,
            self.pane,
            self.viewport,
            widest,
        );
        let selected = row_selection(state.selected(), rows.len());
        let mut items = rows
            .iter()
            .enumerate()
            .map(|(index, row)| {
                ListItem::new(columns.line(row, None)).style(row_style(selected == Some(index)))
            })
            .collect::<Vec<_>>();
        if items.is_empty()
            && let Some(empty) = empty
        {
            items.push(ListItem::new(empty));
        }
        frame.render_stateful_widget(List::new(items), self.inner, state);
    }
}
