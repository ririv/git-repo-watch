//! Shared focus and keyboard behavior for two horizontal confirmation actions.
use crossterm::event::KeyCode;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum Choice {
    #[default]
    Execute,
    Return,
}
pub(crate) enum Input {
    Pending,
    Execute,
    Return,
}
impl Choice {
    pub(crate) fn input(&mut self, key: KeyCode) -> Input {
        match key {
            KeyCode::Left => *self = Self::Execute,
            KeyCode::Right => *self = Self::Return,
            KeyCode::Tab | KeyCode::BackTab => {
                *self = match self {
                    Self::Execute => Self::Return,
                    Self::Return => Self::Execute,
                }
            }
            KeyCode::Enter => {
                return match self {
                    Self::Execute => Input::Execute,
                    Self::Return => Input::Return,
                };
            }
            KeyCode::Esc => return Input::Return,
            _ => {}
        }
        Input::Pending
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn focus_and_acceptance_are_explicit_for_both_actions() {
        let mut choice = Choice::Execute;
        assert!(matches!(choice.input(KeyCode::Right), Input::Pending));
        assert_eq!(choice, Choice::Return);
        assert!(matches!(choice.input(KeyCode::Enter), Input::Return));
        choice.input(KeyCode::Tab);
        assert_eq!(choice, Choice::Execute);
        assert!(matches!(choice.input(KeyCode::Enter), Input::Execute));
        choice.input(KeyCode::BackTab);
        assert_eq!(choice, Choice::Return);
        choice.input(KeyCode::Left);
        assert_eq!(choice, Choice::Execute);
        assert!(matches!(choice.input(KeyCode::Esc), Input::Return));
    }
}
