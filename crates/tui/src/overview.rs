//! A single cancellable local reader with a replaceable view request.
use crate::{Service, spinner};
use engine::{cancel::Cancel, local::overview::Snapshot};
use std::{
    path::{Path, PathBuf},
    sync::{Arc, mpsc},
    thread,
    time::Instant,
};

#[derive(Debug, Default)]
pub(super) enum Content {
    #[default]
    Closed,
    Loading(PathBuf),
    Ready(Box<Snapshot>),
    Refreshing(Box<Snapshot>),
    Failed {
        path: PathBuf,
        error: String,
    },
}
impl Content {
    pub fn path(&self) -> Option<&Path> {
        match self {
            Self::Closed => None,
            Self::Loading(path) | Self::Failed { path, .. } => Some(path),
            Self::Ready(snapshot) | Self::Refreshing(snapshot) => Some(&snapshot.path),
        }
    }
    pub fn snapshot(&self) -> Option<&Snapshot> {
        match self {
            Self::Ready(snapshot) | Self::Refreshing(snapshot) => Some(snapshot),
            _ => None,
        }
    }
    fn requested(&self) -> Option<&Path> {
        match self {
            Self::Loading(path) => Some(path),
            Self::Refreshing(snapshot) => Some(&snapshot.path),
            _ => None,
        }
    }
}

#[derive(Debug)]
struct Reader {
    path: PathBuf,
    cancel: Cancel,
    rx: mpsc::Receiver<anyhow::Result<Snapshot>>,
    handle: thread::JoinHandle<()>,
    animation: spinner::Spinner,
}

#[derive(Debug, Default)]
pub(super) struct Overview {
    pub content: Content,
    reader: Option<Reader>,
}
impl Overview {
    pub fn open(&mut self, path: PathBuf, service: &Arc<Service>) {
        if let Some(reader) = &self.reader {
            reader.cancel.cancel();
        }
        self.content = match std::mem::take(&mut self.content) {
            Content::Ready(snapshot) | Content::Refreshing(snapshot) if snapshot.path == path => {
                Content::Refreshing(snapshot)
            }
            _ => Content::Loading(path),
        };
        self.start(service);
    }
    fn start(&mut self, service: &Arc<Service>) {
        if self.reader.is_some() {
            return;
        }
        let Some(path) = self.content.requested().map(Path::to_owned) else {
            return;
        };
        let task_path = path.clone();
        let cancel = Cancel::default();
        let task_cancel = cancel.clone();
        let service = service.clone();
        let language = i18n::language();
        let (tx, rx) = mpsc::channel();
        let handle = thread::spawn(move || {
            let _language = i18n::using(language);
            let _ = tx.send(service.overview(&task_path, &task_cancel));
        });
        self.reader = Some(Reader {
            path,
            cancel,
            rx,
            handle,
            animation: spinner::Spinner::new(Instant::now()),
        });
    }
    pub fn poll(&mut self, service: &Arc<Service>) -> bool {
        let Some(reader) = &self.reader else {
            return false;
        };
        let result = match reader.rx.try_recv() {
            Ok(result) => result,
            Err(mpsc::TryRecvError::Empty) => return false,
            Err(mpsc::TryRecvError::Disconnected) => Err(anyhow::anyhow!(i18n::tr!(
                BackgroundWorkerTerminatedUnexpectedly
            ))),
        };
        let reader = self.reader.take().expect("completed reader exists");
        let joined = reader.handle.join();
        if !reader.cancel.is_cancelled() && self.content.requested() == Some(&reader.path) {
            let result = if joined.is_err() {
                Err(anyhow::anyhow!(i18n::tr!(BackgroundWorkerPanicked)))
            } else {
                result
            };
            self.content = match result {
                Ok(snapshot) => Content::Ready(Box::new(snapshot)),
                Err(error) => Content::Failed {
                    path: reader.path,
                    error: format!("{error:#}"),
                },
            };
        }
        self.start(service);
        true
    }
    pub fn animate(&mut self) -> bool {
        self.reader
            .as_mut()
            .is_some_and(|reader| reader.animation.tick(Instant::now()))
    }
    pub fn spinner(&self) -> &str {
        self.reader
            .as_ref()
            .map(|reader| reader.animation.symbol())
            .unwrap_or("")
    }
    pub fn shutdown(&mut self) {
        if let Some(reader) = self.reader.take() {
            reader.cancel.cancel();
            let _ = reader.handle.join();
        }
    }
}

impl crate::Screen {
    pub(super) fn refresh_overview(&mut self) {
        if self.page == crate::Page::Detail
            && let Some(path) = self.overview.content.path().map(Path::to_owned)
        {
            self.overview.open(path, &self.service);
        }
    }
}

impl Drop for Overview {
    fn drop(&mut self) {
        self.shutdown();
    }
}
