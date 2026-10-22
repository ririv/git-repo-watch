//! A single cancellable reader coalesces selection changes to the latest request.
use super::Key;
use crate::{Service, document};
use engine::{cancel::Cancel, local::files::Files};
use std::{
    sync::{Arc, mpsc},
    thread,
};

#[derive(Debug)]
struct Request {
    key: Key,
    snapshot: Files,
}
#[derive(Debug)]
struct Reader {
    cancel: Cancel,
    handle: thread::JoinHandle<()>,
    rx: mpsc::Receiver<Result<Box<document::View>, String>>,
    key: Key,
}
#[derive(Debug, Default)]
pub(super) struct Loader {
    reader: Option<Reader>,
    queued: Option<Request>,
}
impl Loader {
    pub fn pending(&self) -> bool {
        self.reader.is_some() || self.queued.is_some()
    }
    pub fn request(&mut self, key: Key, snapshot: Files, service: &Arc<Service>) {
        if let Some(reader) = &self.reader {
            reader.cancel.cancel();
        }
        self.queued = Some(Request { key, snapshot });
        self.start(service);
    }
    pub fn clear(&mut self) {
        self.queued = None;
        if let Some(reader) = &self.reader {
            reader.cancel.cancel();
        }
    }
    fn start(&mut self, service: &Arc<Service>) {
        if self.reader.is_some() {
            return;
        }
        let Some(request) = self.queued.take() else {
            return;
        };
        let service = service.clone();
        let cancel = Cancel::default();
        let task_cancel = cancel.clone();
        let key = request.key.clone();
        let language = i18n::language();
        let (tx, rx) = mpsc::channel();
        let handle = thread::spawn(move || {
            let _language = i18n::using(language);
            let change = &request.key.change;
            let result = service
                .diff(&request.snapshot, change, &task_cancel)
                .and_then(|document| {
                    document::prepare(
                        &change.location.display(),
                        document,
                        change.area,
                        &task_cancel,
                    )
                })
                .map_err(|error| format!("{error:#}"));
            let _ = tx.send(result);
        });
        self.reader = Some(Reader {
            cancel,
            handle,
            rx,
            key,
        });
    }
    pub fn poll(
        &mut self,
        service: &Arc<Service>,
    ) -> Option<(Key, Result<Box<document::View>, String>)> {
        if !self
            .reader
            .as_ref()
            .is_some_and(|reader| reader.handle.is_finished())
        {
            return None;
        }
        let reader = self.reader.take().expect("completed reader");
        let result = if reader.handle.join().is_err() {
            Err(i18n::tr!(BackgroundWorkerPanicked).to_owned())
        } else {
            reader.rx.try_recv().unwrap_or_else(|_| {
                Err(i18n::tr!(BackgroundWorkerTerminatedUnexpectedly).to_owned())
            })
        };
        let result = (!reader.cancel.is_cancelled()).then_some((reader.key, result));
        self.start(service);
        result
    }
}
impl Drop for Loader {
    fn drop(&mut self) {
        self.clear();
        if let Some(reader) = self.reader.take() {
            let _ = reader.handle.join();
        }
    }
}
