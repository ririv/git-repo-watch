//! Recovery choices use the same bottom row as other contextual actions.
use super::super::*;
use crate::{replay::controls, view::input::actions};
use engine::operations::replay::Control;
pub(in crate::view) fn render(
    frame: &mut Frame,
    inner: Rect,
    selected: Control,
    enabled: [bool; 3],
    hits: &mut crate::pointer::Map,
) {
    let items = controls::ACTIONS
        .into_iter()
        .enumerate()
        .map(|(index, control)| actions::Item {
            label: controls::label(control),
            state: actions::State::new(enabled[index], selected == control),
            target: actions::Target::Button(crate::pointer::Action::ReplayControl(control)),
        })
        .collect::<Vec<_>>();
    actions::render(frame, inner, &items, hits);
}
