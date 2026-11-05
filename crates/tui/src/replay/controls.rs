//! Keyboard focus for the native operation recovery controls.
use engine::operations::replay::{Active, Control, State};

pub(crate) const ACTIONS: [Control; 3] = [Control::Continue, Control::Skip, Control::Abort];

#[derive(Debug, Default, Clone)]
pub(crate) enum Focus {
    #[default]
    Content,
    Action(Control),
    CommandAction {
        selected: Control,
        draft: Box<crate::bar::CommandDraft>,
        cursor: usize,
    },
}
impl Focus {
    pub fn selected(&self) -> Option<Control> {
        match self {
            Self::Content => None,
            Self::Action(control)
            | Self::CommandAction {
                selected: control, ..
            } => Some(*control),
        }
    }
    pub fn choose(&mut self, value: Control) {
        match self {
            Self::Action(selected) | Self::CommandAction { selected, .. } => *selected = value,
            Self::Content => *self = Self::Action(value),
        }
    }
    pub fn step(&mut self, backwards: bool) {
        let control = match self {
            Self::Action(control)
            | Self::CommandAction {
                selected: control, ..
            } => control,
            Self::Content => return,
        };
        let index = ACTIONS
            .iter()
            .position(|value| value == control)
            .expect("known recovery action");
        *control = ACTIONS[(index + if backwards { ACTIONS.len() - 1 } else { 1 }) % ACTIONS.len()];
    }
    pub fn refreshed(&self, previous: &State, next: &State) -> Self {
        match (previous, next) {
            (State::Active(a), State::Active(b)) if a.kind == b.kind && a.commit == b.commit => {
                self.clone()
            }
            _ => Self::Content,
        }
    }
}
pub(crate) fn enabled(active: &Active, control: Control) -> bool {
    control != Control::Continue || active.conflicts.is_empty()
}
pub(crate) fn label(control: Control) -> &'static str {
    match control {
        Control::Continue => i18n::tr!(ReplayContinueShort),
        Control::Skip => i18n::tr!(ReplaySkipShort),
        Control::Abort => i18n::tr!(ReplayAbortShort),
    }
}
