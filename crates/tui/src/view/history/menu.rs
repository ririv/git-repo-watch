//! Selected-commit actions share the bottom candidate panel and input border.
use super::*;
use crate::history::menu::PointerInput;

pub(in crate::view) fn render(frame: &mut Frame, screen: &mut Screen, body: Rect) {
    if body.is_empty() || screen.tasks.session.open() {
        return;
    }
    let Some(count) = screen.history_actions().map(|menu| menu.operations().len()) else {
        return;
    };
    let (popup, inner) = super::super::candidates::panel(
        frame,
        screen,
        body,
        count,
        Some(i18n::tr!(HistoryActionsTitle)),
    );
    let Screen {
        mode,
        pointer: hits,
        ..
    } = screen;
    let Mode::History(view) = mode else {
        return;
    };
    let crate::history::View::Graph(session) = view.as_mut() else {
        return;
    };
    let Some(menu) = session.selection.menu_mut() else {
        return;
    };
    hits.button(
        body,
        crate::pointer::Action::HistoryMenu(PointerInput::Close),
    );
    hits.button(
        popup,
        crate::pointer::Action::HistoryMenu(PointerInput::Stay),
    );
    let items = menu
        .operations()
        .iter()
        .map(|operation| ListItem::new(operation.label()))
        .collect::<Vec<_>>();
    frame.render_stateful_widget(
        List::new(items).highlight_style(selected_style()),
        inner,
        &mut menu.state,
    );
    crate::scrollbar::vertical(
        frame,
        popup,
        inner,
        menu.operations().len(),
        menu.state.offset(),
    );
    hits.vertical(
        popup,
        inner,
        menu.operations()
            .len()
            .saturating_sub(inner.height as usize),
        menu.state.offset(),
        crate::pointer::ScrollTarget::HistoryActions,
    );
    for (row, index) in (menu.state.offset()..menu.operations().len())
        .take(inner.height as usize)
        .enumerate()
    {
        hits.button(
            Rect::new(inner.x, inner.y + row as u16, inner.width, 1),
            crate::pointer::Action::HistoryMenu(PointerInput::Choose(index)),
        );
    }
}
