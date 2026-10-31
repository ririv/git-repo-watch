//! Typed display preferences and independent configuration edits.
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Preferences {
    pub file_layout: FileLayout,
    pub diff_layout: DiffLayout,
    pub diff_header: HeaderVisibility,
    pub repository_order: RepositoryOrder,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileLayout {
    #[default]
    Tree,
    Flat,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiffLayout {
    #[default]
    Auto,
    Split,
    Unified,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HeaderVisibility {
    #[default]
    Collapsed,
    Expanded,
}
impl HeaderVisibility {
    pub fn toggled(self) -> Self {
        match self {
            Self::Collapsed => Self::Expanded,
            Self::Expanded => Self::Collapsed,
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RepositoryOrder {
    #[default]
    Priority,
    Name,
    Path,
    Updated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edit {
    FileLayout(FileLayout),
    DiffLayout(DiffLayout),
    DiffHeader(HeaderVisibility),
    RepositoryOrder(RepositoryOrder),
}
impl Edit {
    pub fn apply(self, preferences: &mut Preferences) {
        match self {
            Self::FileLayout(value) => preferences.file_layout = value,
            Self::DiffLayout(value) => preferences.diff_layout = value,
            Self::DiffHeader(value) => preferences.diff_header = value,
            Self::RepositoryOrder(value) => preferences.repository_order = value,
        }
    }
    pub fn same_field(self, other: Self) -> bool {
        matches!(
            (self, other),
            (Self::FileLayout(_), Self::FileLayout(_))
                | (Self::DiffLayout(_), Self::DiffLayout(_))
                | (Self::DiffHeader(_), Self::DiffHeader(_))
                | (Self::RepositoryOrder(_), Self::RepositoryOrder(_))
        )
    }
}
