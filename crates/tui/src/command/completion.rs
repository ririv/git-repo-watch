//! Parameter completion is separate from executable command entries.
use super::path;
use crate::Editor;

#[derive(Debug, Default)]
pub(crate) enum Menu {
    #[default]
    Commands,
    Directories(path::Directories),
    Options(super::options::Candidates),
}
pub(crate) trait Selection {
    fn advance(&mut self, backwards: bool);
    fn choose(&self) -> Editor;
}
impl Selection for path::Directories {
    fn advance(&mut self, backwards: bool) {
        self.advance(backwards);
    }
    fn choose(&self) -> Editor {
        self.choose()
    }
}
impl Menu {
    pub fn selection(&mut self) -> Option<&mut dyn Selection> {
        match self {
            Self::Commands | Self::Options(_) => None,
            Self::Directories(values) => Some(values),
        }
    }
}
pub(crate) fn complete(editor: &mut Editor, home: &std::path::Path) -> anyhow::Result<Menu> {
    path::complete(editor, home)
}
