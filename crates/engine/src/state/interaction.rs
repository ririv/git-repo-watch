//! Persisted ownership of pointer input in the terminal.
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InteractionMode {
    #[default]
    Application,
    TextSelection,
}
impl InteractionMode {
    pub fn next(self) -> Self {
        match self {
            Self::Application => Self::TextSelection,
            Self::TextSelection => Self::Application,
        }
    }
}
