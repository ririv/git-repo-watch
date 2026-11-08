//! A single cancellable reader coalesces selection changes to the latest request.
use super::Key;
#[derive(Debug)]
pub(super) struct Review {
    pub content: Box<document::View>,
    pub editing: super::edit::State,
}
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
    rx: mpsc::Receiver<Result<Review, String>>,
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
                    let editing =
                        super::edit::State::new(engine::local::files::partial::Snapshot::capture(
                            &request.snapshot,
                            change,
                            &document,
                            &task_cancel,
                        ));
                    let content = if change.kind == 'U' {
                        let snapshot = service.conflict(&request.snapshot, change, &task_cancel)?;
                        let pairs = crate::conflict::pair::Pairs::prepare(
                            &snapshot,
                            &document,
                            &task_cancel,
                        )?;
                        let result = crate::conflict::result::View::prepare(
                            document,
                            change,
                            request.key.theme,
                            &task_cancel,
                        )?;
                        Box::new(document::View::Conflict(Box::new(
                            crate::conflict::View::prepare_with_theme(
                                snapshot,
                                result,
                                pairs,
                                request.key.theme,
                                &task_cancel,
                            )?,
                        )))
                    } else {
                        let mut content = document::prepare_change_with_theme(
                            &change.location.display(),
                            document,
                            change,
                            request.key.theme,
                            &task_cancel,
                        )?;
                        if let document::View::Text(view) = content.as_mut() {
                            view.interaction = match &editing {
                                super::edit::State::Ready(_) => {
                                    crate::diff::Interaction::Editable(None)
                                }
                                super::edit::State::Unavailable(_) => {
                                    crate::diff::Interaction::ReadOnly
                                }
                            };
                        }
                        content
                    };
                    Ok(Review { content, editing })
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
    pub fn poll(&mut self, service: &Arc<Service>) -> Option<(Key, Result<Review, String>)> {
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
