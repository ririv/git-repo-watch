//! Conflict choices use the shared bottom action row.
use super::*;
use crate::conflict::controls::Action;
pub(super) fn row(screen: &Screen, selected: Action) -> actions::Row {
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
        .collect();
    actions::Row::new(items, actions::Spacing::Flush)
}
