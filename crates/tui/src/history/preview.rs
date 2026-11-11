//! A bounded background reader supplies file-level changes for the selected graph entry.
use crate::{loading::Feedback, reader::Worker};
use engine::{
    local::history::{Change, Oid, Parent},
    service::Service,
};
use ratatui::layout::Rect;
use std::{path::PathBuf, sync::Arc, time::Instant};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Key {
    pub path: PathBuf,
    pub oid: Oid,
}
#[derive(Debug, Default)]
pub(crate) enum Content {
    #[default]
    Hidden,
    Loading(Key),
    Ready {
        key: Key,
        changes: Vec<Change>,
    },
    Failed {
        key: Key,
        error: String,
    },
}
impl Content {
    fn key(&self) -> Option<&Key> {
        match self {
            Self::Hidden => None,
            Self::Loading(key) | Self::Ready { key, .. } | Self::Failed { key, .. } => Some(key),
        }
    }
}
#[derive(Debug, Default)]
pub(crate) struct Preview {
    pub content: Content,
    pub area: Option<Rect>,
    pub offset: usize,
    pub maximum: usize,
    pub feedback: Feedback<()>,
    reader: Worker<Key, Vec<Change>>,
}
impl Preview {
    pub fn select(&mut self, key: Key, area: Rect) {
        self.area = Some(area);
        if self.content.key() == Some(&key) {
            return;
        }
        self.reader.clear();
        self.content = Content::Loading(key);
        self.offset = 0;
        self.maximum = 0;
        self.feedback.begin(None, Instant::now());
    }
    pub fn hide(&mut self) {
        self.reader.clear();
        self.content = Content::Hidden;
        self.area = None;
        self.offset = 0;
        self.maximum = 0;
        self.feedback.finish();
    }
    pub fn scroll(&mut self, delta: isize) {
        self.offset = self.offset.saturating_add_signed(delta).min(self.maximum);
    }
    pub fn poll(&mut self, service: &Arc<Service>) -> bool {
        if let Content::Loading(key) = &self.content
            && self.reader.requested() != Some(key)
        {
            let key = key.clone();
            let request = key.clone();
            let service = service.clone();
            self.reader.request(key, move |cancel| {
                service
                    .history_commit(&request.path, request.oid, Parent::First, &cancel)
                    .map(|detail| detail.changes)
            });
        }
        let mut changed = self.feedback.tick(Instant::now());
        if let Some((key, result)) = self.reader.poll()
            && self.content.key() == Some(&key)
        {
            self.content = match result {
                Ok(changes) => Content::Ready { key, changes },
                Err(error) => Content::Failed { key, error },
            };
            self.feedback.finish();
            changed = true;
        }
        changed
    }
}
