//! Tagged immutable sources keep stash layers separate from ordinary commit parents.
use anyhow::Result;
use engine::{
    cancel::Cancel,
    local::{files::patch::Document, history, stash},
    service::Service,
};
use std::{path::Path, sync::Arc};

#[derive(Debug, Clone)]
pub(crate) enum Source {
    Commit(Arc<history::Detail>),
    Stash(Arc<stash::preview::Detail>),
}
impl Source {
    pub fn path(&self) -> &Path {
        match self {
            Self::Commit(detail) => &detail.path,
            Self::Stash(detail) => &detail.path,
        }
    }
    pub fn oid(&self) -> history::Oid {
        match self {
            Self::Commit(detail) => detail.oid,
            Self::Stash(detail) => detail.entry.oid,
        }
    }
    pub fn len(&self) -> usize {
        match self {
            Self::Commit(detail) => detail.changes.len(),
            Self::Stash(detail) => detail.files.len(),
        }
    }
    pub fn change(&self, index: usize) -> &history::Change {
        match self {
            Self::Commit(detail) => &detail.changes[index],
            Self::Stash(detail) => &detail.files[index].change,
        }
    }
    pub fn changes(&self) -> Vec<&history::Change> {
        (0..self.len()).map(|i| self.change(i)).collect()
    }
    pub fn empty_message(&self, query: &str) -> &'static str {
        match (self, query.is_empty()) {
            (Self::Commit(_), true) => i18n::tr!(HistoryNoChanges),
            (Self::Stash(_), true) => i18n::tr!(StashPreviewEmpty),
            (Self::Commit(_), false) => i18n::tr!(HistoryNoFilesMatch),
            (Self::Stash(_), false) => i18n::tr!(StashPreviewNoMatch),
        }
    }
    pub fn message_title(&self) -> &'static str {
        match self {
            Self::Commit(_) => i18n::tr!(HistoryMessage),
            Self::Stash(_) => i18n::tr!(StashDetails),
        }
    }
    pub fn message(&self) -> &str {
        match self {
            Self::Commit(detail) => detail.message.text(),
            Self::Stash(detail) => &detail.entry.message,
        }
    }
    pub fn comparison(&self, index: usize) -> crate::diff::Comparison {
        match self {
            Self::Commit(detail) => crate::diff::Comparison::History {
                before: detail.base,
                after: detail.oid,
            },
            Self::Stash(detail) => crate::diff::Comparison::Stash(detail.files[index].layer),
        }
    }
    pub fn document(&self, service: &Service, index: usize, cancel: &Cancel) -> Result<Document> {
        match self {
            Self::Commit(detail) => service.history_diff(detail, &detail.changes[index], cancel),
            Self::Stash(detail) => service.stash_diff(detail, index, cancel),
        }
    }
}
