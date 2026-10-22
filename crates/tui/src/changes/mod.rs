//! One changes workspace composes file selection and a versioned background preview.
mod load;
use crate::{FileSession, Mode, Screen, Service, document, spinner, workspace::Arrangement};
use engine::local::files::{Change, Files};
use std::{sync::Arc, time::Instant};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Key {
    revision: u64,
    change: Change,
}
#[derive(Debug)]
pub(super) enum Preview {
    Empty,
    Loading(Key),
    Ready {
        key: Key,
        content: Box<document::View>,
        scroll: usize,
    },
    Failed {
        key: Key,
        error: String,
    },
}
impl Preview {
    fn key(&self) -> Option<&Key> {
        match self {
            Self::Empty => None,
            Self::Loading(key) | Self::Ready { key, .. } | Self::Failed { key, .. } => Some(key),
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Focus {
    Files,
    Preview,
}
#[derive(Debug, Clone, Copy)]
pub(super) enum ReturnTo {
    Repositories,
    Files,
}
#[derive(Debug)]
pub(super) struct Session {
    pub files: FileSession,
    pub return_to: ReturnTo,
    pub focus: Focus,
    pub arrangement: Arrangement,
    pub preview: Preview,
    revision: u64,
    loader: load::Loader,
    animation: spinner::Spinner,
}
impl Session {
    pub fn new(files: FileSession, return_to: ReturnTo) -> Self {
        Self {
            files,
            return_to,
            focus: Focus::Files,
            arrangement: Arrangement::Tiled,
            preview: Preview::Empty,
            revision: 0,
            loader: Default::default(),
            animation: spinner::Spinner::new(Instant::now()),
        }
    }
    pub fn refreshed(&mut self, snapshot: Files) {
        self.files = self.files.refreshed(snapshot);
        self.revision += 1;
    }
    pub fn pending(&self) -> bool {
        self.loader.pending()
    }
    pub fn symbol(&self) -> &'static str {
        self.animation.symbol()
    }
    pub fn poll(&mut self, service: &Arc<Service>) -> bool {
        let next = self.files.rows().get(self.files.cursor).map(|change| Key {
            revision: self.revision,
            change: (*change).clone(),
        });
        let mut changed = false;
        if self.preview.key() != next.as_ref() {
            match &next {
                Some(key) => {
                    self.loader
                        .request(key.clone(), self.files.snapshot.clone(), service);
                    self.preview = Preview::Loading(key.clone());
                }
                None => {
                    self.loader.clear();
                    self.preview = Preview::Empty;
                }
            }
            changed = true;
        }
        if let Some((key, result)) = self.loader.poll(service)
            && self.preview.key() == Some(&key)
        {
            self.preview = match result {
                Ok(content) => Preview::Ready {
                    key,
                    content,
                    scroll: 0,
                },
                Err(error) => Preview::Failed { key, error },
            };
            changed = true;
        }
        if self.pending() {
            changed |= self.animation.tick(Instant::now());
        }
        changed
    }
    #[cfg(test)]
    pub fn content(&self) -> &document::View {
        match &self.preview {
            Preview::Ready { content, .. } => content,
            other => panic!("preview is not ready: {other:?}"),
        }
    }
}
impl Screen {
    pub(super) fn poll_changes(&mut self) -> bool {
        match &mut self.mode {
            Mode::Changes(session) => session.poll(&self.service),
            _ => false,
        }
    }
}
