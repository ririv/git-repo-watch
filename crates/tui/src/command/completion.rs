//! Parameter completion is separate from executable command entries.
use super::path;
use crate::Editor;

#[derive(Debug, Default)]
pub(crate) enum Menu {
    #[default]
    Commands,
    History(super::history::Session),
    Directories(path::Directories),
    Options(super::options::Candidates),
    Values(super::values::Candidates),
    Loading(super::loader::Request),
    Failed(String),
}
pub(crate) trait Selection {
    fn advance(&mut self, backwards: bool);
    fn choose(&self) -> Option<Editor>;
    fn unavailable(&self) -> Option<&str> {
        None
    }
}
impl Selection for path::Directories {
    fn advance(&mut self, backwards: bool) {
        self.advance(backwards);
    }
    fn choose(&self) -> Option<Editor> {
        self.choose()
    }
}
impl Selection for super::values::Candidates {
    fn advance(&mut self, backwards: bool) {
        self.advance(backwards);
    }
    fn choose(&self) -> Option<Editor> {
        self.choose()
    }
    fn unavailable(&self) -> Option<&str> {
        self.unavailable()
    }
}
impl Menu {
    pub fn selection(&mut self) -> Option<&mut dyn Selection> {
        match self {
            Self::Commands
            | Self::History(_)
            | Self::Options(_)
            | Self::Loading(_)
            | Self::Failed(_) => None,
            Self::Directories(values) => Some(values),
            Self::Values(values) => Some(values),
        }
    }
}
