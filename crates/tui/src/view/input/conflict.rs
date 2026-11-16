//! Conflict choices use the shared bottom action row.
use super::*;
use crate::conflict::controls::Action;
pub(super) fn render(frame: &mut Frame, screen: &mut Screen, inner: Rect, selected: Action) {
    let items = Action::ALL
        .into_iter()
        .map(|action| actions::Item {
            label: action.label(),
            state: actions::State::new(
                screen.conflict_control_reason(action).is_none(),
                action == selected,
            ),
            target: actions::Target::Button(crate::pointer::Action::ConflictControl(action)),
        })
        .collect::<Vec<_>>();
    actions::render(frame, inner, &items, &mut screen.pointer);
}
