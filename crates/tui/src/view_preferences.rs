//! Adapt persisted view choices without coupling storage types to terminal layout.
mod writer;
use crate::{Mode, Screen};
use engine::state::view::{DiffLayout, Edit, FileLayout, RepositoryOrder};
pub(crate) use writer::Writer;

impl From<FileLayout> for crate::files::Layout {
    fn from(value: FileLayout) -> Self {
        match value {
            FileLayout::Tree => Self::Tree,
            FileLayout::Flat => Self::Flat,
        }
    }
}
impl From<crate::files::Layout> for FileLayout {
    fn from(value: crate::files::Layout) -> Self {
        match value {
            crate::files::Layout::Tree => Self::Tree,
            crate::files::Layout::Flat => Self::Flat,
        }
    }
}
impl From<DiffLayout> for crate::diff::Preference {
    fn from(value: DiffLayout) -> Self {
        match value {
            DiffLayout::Auto => Self::Auto,
            DiffLayout::Split => Self::Split,
            DiffLayout::Unified => Self::Unified,
        }
    }
}
impl From<crate::diff::Preference> for DiffLayout {
    fn from(value: crate::diff::Preference) -> Self {
        match value {
            crate::diff::Preference::Auto => Self::Auto,
            crate::diff::Preference::Split => Self::Split,
            crate::diff::Preference::Unified => Self::Unified,
        }
    }
}
impl From<RepositoryOrder> for crate::repository_list::Order {
    fn from(value: RepositoryOrder) -> Self {
        match value {
            RepositoryOrder::Priority => Self::Priority,
            RepositoryOrder::Name => Self::Name,
            RepositoryOrder::Path => Self::Path,
            RepositoryOrder::Updated => Self::Updated,
        }
    }
}
impl From<crate::repository_list::Order> for RepositoryOrder {
    fn from(value: crate::repository_list::Order) -> Self {
        match value {
            crate::repository_list::Order::Priority => Self::Priority,
            crate::repository_list::Order::Name => Self::Name,
            crate::repository_list::Order::Path => Self::Path,
            crate::repository_list::Order::Updated => Self::Updated,
        }
    }
}
impl Screen {
    pub(crate) fn save_view_preference(&mut self, edit: Edit) {
        self.view_save.submit(self.service.clone(), edit);
    }
    pub(crate) fn set_file_layout(&mut self, layout: crate::files::Layout) {
        match &mut self.mode {
            Mode::Files(files) => files.set_layout(layout),
            Mode::Changes(session) => session.files.set_layout(layout),
            _ => {}
        }
        self.file_layout = layout;
        self.save_view_preference(Edit::FileLayout(layout.into()));
    }
    pub(crate) fn set_diff_layout(&mut self, preference: crate::diff::Preference) {
        self.diff_preference = preference;
        self.save_view_preference(Edit::DiffLayout(preference.into()));
        self.message =
            crate::message::Message::information(i18n::tf!(DiffLayoutSelected, preference.label()));
    }
    pub(crate) fn set_diff_context(&mut self, context: crate::diff::Context) {
        self.diff_context = context;
        self.save_view_preference(Edit::DiffContext(context));
    }
    pub(crate) fn poll_view_preferences(&mut self) -> bool {
        let Some((edit, result)) = self.view_save.poll() else {
            return false;
        };
        match result {
            Ok(()) => edit.apply(&mut self.config.view),
            Err(error) => {
                self.message = crate::message::Message::error(format!(
                    "{}: {error:#}",
                    i18n::tr!(ViewPreferenceSaveFailed)
                ))
            }
        }
        true
    }
}
