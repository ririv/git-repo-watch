//! Recovery choices use the same bottom row as other contextual actions.
use crate::{replay::controls, view::input::actions};
use engine::operations::replay::{Active, Control};
pub(in crate::view) fn row(operation: &Active, selected: Control) -> actions::Row {
    let items = controls::ACTIONS
        .into_iter()
        .map(|control| actions::Item {
            label: controls::label(control),
            state: actions::State::new(controls::enabled(operation, control), selected == control),
            target: actions::Target::Button(crate::pointer::Action::ReplayControl(control)),
        })
        .collect();
    actions::Row::new(items, actions::Spacing::Flush)
}
