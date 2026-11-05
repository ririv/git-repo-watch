//! One cancellable reader populates repository panes without blocking interaction.
use crate::{FileSession, Service, history, spinner};
use engine::{cancel::Cancel, local::history::Request};
use std::{
    path::PathBuf,
    sync::{Arc, mpsc},
    thread,
    time::Instant,
};

pub(super) enum Event {
    Branches(Result<crate::branches::Session, String>),
    Files(Result<Box<FileSession>, String>),
    History(Result<Box<history::View>, String>),
}
#[derive(Debug, Clone, Copy)]
enum Phase {
    Files,
    Branches,
    History,
    Complete,
}
#[derive(Debug)]
struct Reader {
    path: PathBuf,
    cancel: Cancel,
    rx: mpsc::Receiver<Event>,
    handle: thread::JoinHandle<()>,
    phase: Phase,
    animation: spinner::Spinner,
}
#[derive(Debug, Default)]
pub(super) struct Loader {
    reader: Option<Reader>,
    queued: Option<PathBuf>,
}
impl Loader {
    pub fn pending(&self) -> bool {
        self.reader.is_some() || self.queued.is_some()
    }
    pub fn request(&mut self, path: PathBuf, service: &Arc<Service>) {
        if let Some(reader) = &self.reader {
            reader.cancel.cancel();
        }
        self.queued = Some(path);
        self.start(service);
    }
    fn start(&mut self, service: &Arc<Service>) {
        if self.reader.is_some() {
            return;
        }
        let Some(path) = self.queued.take() else {
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
            let files = service
                .files(&task_path, &task_cancel)
                .map(FileSession::new)
                .map(Box::new)
                .map_err(|error| format!("{error:#}"));
            let _ = tx.send(Event::Files(files));
            if task_cancel.is_cancelled() {
                return;
            }
            let branches = service
                .branches(&task_path, &task_cancel)
                .map(crate::branches::Session::new)
                .map_err(|error| format!("{error:#}"));
            let _ = tx.send(Event::Branches(branches));
            if task_cancel.is_cancelled() {
                return;
            }
            let history = service
                .history(&task_path, Request::Start, &task_cancel)
                .and_then(|page| history::Session::new(page, &task_cancel))
                .map(|session| Box::new(history::View::Graph(session)))
                .map_err(|error| format!("{error:#}"));
            let _ = tx.send(Event::History(history));
        });
        self.reader = Some(Reader {
            path,
            cancel,
            rx,
            handle,
            phase: Phase::Files,
            animation: spinner::Spinner::new(Instant::now()),
        });
    }
    pub fn poll(&mut self, service: &Arc<Service>) -> Vec<(PathBuf, Event)> {
        // 先记录完成状态，再排空消息，避免线程在两步之间退出时丢失最后一条结果。
        let complete = self
            .reader
            .as_ref()
            .is_some_and(|reader| reader.handle.is_finished());
        let mut events = Vec::new();
        if let Some(reader) = &mut self.reader {
            while let Ok(event) = reader.rx.try_recv() {
                reader.phase = match event {
                    Event::Files(_) => Phase::Branches,
                    Event::Branches(_) => Phase::History,
                    Event::History(_) => Phase::Complete,
                };
                if !reader.cancel.is_cancelled() {
                    events.push((reader.path.clone(), event));
                }
            }
        }
        if complete {
            let reader = self.reader.take().expect("finished reader exists");
            if reader.handle.join().is_err() && !reader.cancel.is_cancelled() {
                let error = i18n::tr!(BackgroundWorkerPanicked).to_owned();
                if matches!(reader.phase, Phase::Files) {
                    events.push((reader.path.clone(), Event::Files(Err(error.clone()))));
                }
                if matches!(reader.phase, Phase::Files | Phase::Branches) {
                    events.push((reader.path.clone(), Event::Branches(Err(error.clone()))));
                }
                if !matches!(reader.phase, Phase::Complete) {
                    events.push((reader.path, Event::History(Err(error))));
                }
            }
            self.start(service);
        }
        events
    }
    pub fn animate(&mut self) -> bool {
        self.reader
            .as_mut()
            .is_some_and(|reader| reader.animation.tick(Instant::now()))
    }
    pub fn spinner(&self) -> &'static str {
        self.reader
            .as_ref()
            .map(|reader| reader.animation.symbol())
            .unwrap_or("⠋")
    }
    pub fn shutdown(&mut self) {
        self.queued = None;
        if let Some(reader) = self.reader.take() {
            reader.cancel.cancel();
            let _ = reader.handle.join();
        }
    }
}
impl Drop for Loader {
    fn drop(&mut self) {
        self.shutdown();
    }
}
