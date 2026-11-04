//! One bounded channel streams immutable file diffs; saved views share the same reader.
use crate::inspection::Source;
use crate::{Service, diff, loading::Feedback, spinner::Spinner};
use engine::cancel::Cancel;
use std::{
    sync::{Arc, mpsc},
    thread,
    time::Instant,
};

#[derive(Debug, Clone)]
pub(super) enum Entry {
    Pending,
    Ready(Arc<diff::View>),
    Failed(Arc<str>),
}
#[derive(Debug, Default)]
enum State {
    #[default]
    Idle,
    Reading(Reader),
    Complete,
}
#[derive(Debug)]
struct Reader {
    cancel: Cancel,
    rx: mpsc::Receiver<(usize, Result<Arc<diff::View>, String>)>,
    handle: thread::JoinHandle<()>,
}
impl Reader {
    fn close(self) -> Result<(), String> {
        self.cancel.cancel();
        // 先断开接收端，释放因页面暂停而阻塞的发送，再等待线程退出。
        drop(self.rx);
        self.handle
            .join()
            .map_err(|_| i18n::tr!(BackgroundWorkerPanicked).to_owned())
    }
}
#[derive(Debug)]
pub(super) struct Bundle {
    detail: Source,
    pub entries: Vec<Entry>,
    state: State,
    feedback: Feedback<()>,
    animation: Spinner,
}
impl Bundle {
    pub fn new(detail: Source) -> Self {
        let count = detail.len();
        Self {
            detail,
            entries: vec![Entry::Pending; count],
            state: if count == 0 {
                State::Complete
            } else {
                State::Idle
            },
            feedback: Feedback::default(),
            animation: Spinner::new(Instant::now()),
        }
    }
    #[cfg(test)]
    pub fn loaded(&self) -> bool {
        matches!(self.state, State::Complete)
    }
    pub fn symbol(&self) -> Option<&'static str> {
        self.feedback.visible().then(|| self.animation.symbol())
    }
    pub fn poll(&mut self, service: &Arc<Service>) -> bool {
        let mut changed = false;
        if matches!(self.state, State::Idle) {
            let service = service.clone();
            let detail = self.detail.clone();
            let cancel = Cancel::default();
            let worker_cancel = cancel.clone();
            let (tx, rx) = mpsc::sync_channel(4);
            let language = i18n::language();
            let handle = thread::spawn(move || {
                let _language = i18n::using(language);
                for index in 0..detail.len() {
                    if worker_cancel.is_cancelled() {
                        break;
                    }
                    let result = detail
                        .document(&service, index, &worker_cancel)
                        .and_then(|document| {
                            diff::prepare_inline_comparison(
                                document,
                                detail.comparison(index),
                                &worker_cancel,
                            )
                        })
                        .map(|view| Arc::from(*view))
                        .map_err(|error| format!("{error:#}"));
                    if worker_cancel.is_cancelled() || tx.send((index, result)).is_err() {
                        break;
                    }
                }
            });
            self.state = State::Reading(Reader { cancel, rx, handle });
            self.feedback.begin(None, Instant::now());
            changed = true;
        }
        if let State::Reading(reader) = &self.state {
            let mut disconnected = false;
            // 每帧限制接收量，读取大量小文件也不会长期占用界面线程。
            for _ in 0..8 {
                match reader.rx.try_recv() {
                    Ok((index, result)) => {
                        self.entries[index] = match result {
                            Ok(view) => Entry::Ready(view),
                            Err(error) => Entry::Failed(error.into()),
                        };
                        changed = true;
                    }
                    Err(mpsc::TryRecvError::Empty) => break,
                    Err(mpsc::TryRecvError::Disconnected) => {
                        disconnected = true;
                        break;
                    }
                }
            }
            if reader.handle.is_finished() {
                // 线程退出后仍可能有排队的数据，下一帧继续接收，直到全部条目完成。
                let all_received = self
                    .entries
                    .iter()
                    .all(|entry| !matches!(entry, Entry::Pending));
                if all_received || disconnected {
                    let State::Reading(reader) =
                        std::mem::replace(&mut self.state, State::Complete)
                    else {
                        unreachable!()
                    };
                    let error = reader.close().err().unwrap_or_else(|| {
                        i18n::tr!(BackgroundWorkerTerminatedUnexpectedly).to_owned()
                    });
                    for entry in &mut self.entries {
                        if matches!(entry, Entry::Pending) {
                            *entry = Entry::Failed(error.clone().into());
                        }
                    }
                    self.feedback.finish();
                    changed = true;
                }
            }
        }
        if matches!(self.state, State::Reading(_)) {
            changed |= self.feedback.tick(Instant::now());
            if self.feedback.visible() {
                changed |= self.animation.tick(Instant::now());
            }
        }
        changed
    }
}
impl Drop for Bundle {
    fn drop(&mut self) {
        if let State::Reading(reader) = std::mem::replace(&mut self.state, State::Complete) {
            let _ = reader.close();
        }
    }
}
