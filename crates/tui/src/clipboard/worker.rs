//! A single clipboard owner serializes writes and retains Linux selection ownership.
use super::{Payload, Target};
use anyhow::{Context, Result, ensure};
use std::{sync::mpsc, thread};

pub(super) trait Sink {
    fn write(&mut self, text: String) -> Result<()>;
}
struct Native(Option<arboard::Clipboard>);
impl Sink for Native {
    fn write(&mut self, text: String) -> Result<()> {
        if self.0.is_none() {
            self.0 = Some(arboard::Clipboard::new().context(i18n::tr!(ClipboardOpenFailed))?);
        }
        self.0
            .as_mut()
            .expect("clipboard initialized")
            .set_text(text)
            .context(i18n::tr!(ClipboardWriteFailed))
    }
}
enum Message {
    Copy(String),
    Stop,
}
enum Transfer {
    Idle,
    Writing(Target),
}
pub(super) struct Worker {
    tx: mpsc::Sender<Message>,
    rx: mpsc::Receiver<Result<(), String>>,
    transfer: Transfer,
    handle: Option<thread::JoinHandle<()>>,
}
impl Worker {
    pub fn native() -> Self {
        Self::spawn(|| Native(None))
    }
    pub fn spawn<S: Sink + 'static>(factory: impl FnOnce() -> S + Send + 'static) -> Self {
        let (tx, requests) = mpsc::channel();
        let (results, rx) = mpsc::channel();
        let language = i18n::language();
        let handle = thread::spawn(move || {
            let _language = i18n::using(language);
            let mut sink = factory();
            while let Ok(Message::Copy(text)) = requests.recv() {
                let result = sink.write(text).map_err(|error| format!("{error:#}"));
                if results.send(result).is_err() {
                    break;
                }
            }
        });
        Self {
            tx,
            rx,
            transfer: Transfer::Idle,
            handle: Some(handle),
        }
    }
    pub fn submit(&mut self, payload: Payload) -> Result<()> {
        ensure!(
            matches!(self.transfer, Transfer::Idle),
            i18n::tr!(ClipboardBusy)
        );
        self.tx
            .send(Message::Copy(payload.text))
            .context(i18n::tr!(ClipboardWorkerStopped))?;
        self.transfer = Transfer::Writing(payload.target);
        Ok(())
    }
    pub fn poll(&mut self) -> Option<(Target, Result<(), String>)> {
        let Transfer::Writing(target) = self.transfer else {
            return None;
        };
        let result = match self.rx.try_recv() {
            Ok(result) => result,
            Err(mpsc::TryRecvError::Empty) => return None,
            Err(mpsc::TryRecvError::Disconnected) => Err(i18n::tr!(ClipboardWorkerStopped).into()),
        };
        self.transfer = Transfer::Idle;
        Some((target, result))
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        let _ = self.tx.send(Message::Stop);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

#[derive(Default)]
pub(crate) struct Writer {
    worker: Option<Worker>,
}
impl Writer {
    pub fn submit(&mut self, payload: Payload) -> Result<()> {
        self.worker
            .get_or_insert_with(Worker::native)
            .submit(payload)
    }
    pub fn poll(&mut self) -> Option<(Target, Result<(), String>)> {
        self.worker.as_mut()?.poll()
    }
    pub fn shutdown(&mut self) {
        self.worker.take();
    }
    #[cfg(test)]
    pub(crate) fn with_sink(sink: impl FnMut(String) -> Result<()> + Send + 'static) -> Self {
        struct Function<F>(F);
        impl<F: FnMut(String) -> Result<()>> Sink for Function<F> {
            fn write(&mut self, text: String) -> Result<()> {
                (self.0)(text)
            }
        }
        Self {
            worker: Some(Worker::spawn(move || Function(sink))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "requires an isolated desktop clipboard; CI supplies Xvfb on Linux"]
    fn native_clipboard_roundtrip() {
        let text = "clipboard test · 中文\ncomplete content\n";
        let mut sink = Native(None);
        sink.write(text.into()).unwrap();
        // 写入对象仍存活；另一个客户端才能在 Linux 请求并收到其持有的内容。
        let mut reader = arboard::Clipboard::new().unwrap();
        assert_eq!(reader.get_text().unwrap(), text);
    }
}
