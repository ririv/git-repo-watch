//! Keyboard focus for the native operation recovery controls.
use engine::operations::replay::{Active, Control, State};

pub(crate) const ACTIONS: [Control; 3] = [Control::Continue, Control::Skip, Control::Abort];

#[derive(Debug, Default, Clone)]
pub(crate) enum Focus {
    #[default]
    Content,
    Action(Control),
    Conflict(crate::conflict::controls::Action),
    CommandConflict {
        selected: crate::conflict::controls::Action,
        draft: Box<crate::bar::CommandDraft>,
        cursor: usize,
    },
    CommandAction {
        selected: Control,
        draft: Box<crate::bar::CommandDraft>,
        cursor: usize,
    },
}
impl Focus {
    pub fn page(&self) -> crate::input_pages::Page {
        match self {
            Self::Content => crate::input_pages::Page::Input,
            Self::Conflict(_) | Self::CommandConflict { .. } => crate::input_pages::Page::Conflict,
            Self::Action(_) | Self::CommandAction { .. } => crate::input_pages::Page::Replay,
        }
    }
    pub fn active(&self) -> bool {
        !matches!(self, Self::Content)
    }
    pub fn conflict(&self) -> Option<crate::conflict::controls::Action> {
        match self {
            Self::Conflict(action)
            | Self::CommandConflict {
                selected: action, ..
            } => Some(*action),
            _ => None,
        }
    }
    pub fn choose_conflict(&mut self, action: crate::conflict::controls::Action) {
        match self {
            Self::Conflict(selected) | Self::CommandConflict { selected, .. } => *selected = action,
            _ => *self = Self::Conflict(action),
        }
    }
    pub fn selected(&self) -> Option<Control> {
        match self {
            Self::Content | Self::Conflict(_) | Self::CommandConflict { .. } => None,
            Self::Action(control)
            | Self::CommandAction {
                selected: control, ..
            } => Some(*control),
        }
    }
    pub fn choose(&mut self, value: Control) {
        match self {
            Self::Action(selected) | Self::CommandAction { selected, .. } => *selected = value,
            Self::Content | Self::Conflict(_) | Self::CommandConflict { .. } => {
                *self = Self::Action(value)
            }
        }
    }
    pub fn step(&mut self, backwards: bool) {
        let control = match self {
            Self::Action(control)
            | Self::CommandAction {
                selected: control, ..
            } => control,
            Self::Content | Self::Conflict(_) | Self::CommandConflict { .. } => return,
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
            (State::Other, State::Other) => self.clone(),
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
