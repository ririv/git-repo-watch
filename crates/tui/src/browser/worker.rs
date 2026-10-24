//! A bounded reader coalesces requests and discards cancelled results.
use engine::cancel::Cancel;
use std::{fmt, sync::mpsc, thread};
type Work<T> = Box<dyn FnOnce(Cancel) -> anyhow::Result<T> + Send>;
struct Job<K, T> {
    key: K,
    work: Work<T>,
}
struct Active<K, T> {
    key: K,
    cancel: Cancel,
    handle: thread::JoinHandle<()>,
    rx: mpsc::Receiver<Result<T, String>>,
}
pub(super) struct Worker<K, T> {
    active: Option<Active<K, T>>,
    queued: Option<Job<K, T>>,
}
impl<K, T> Default for Worker<K, T> {
    fn default() -> Self {
        Self {
            active: None,
            queued: None,
        }
    }
}
impl<K, T> fmt::Debug for Worker<K, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Worker")
            .field("pending", &self.pending())
            .finish()
    }
}
impl<K, T> Worker<K, T> {
    pub fn requested(&self) -> Option<&K> {
        self.queued
            .as_ref()
            .map(|job| &job.key)
            .or_else(|| self.active.as_ref().map(|active| &active.key))
    }
    pub fn pending(&self) -> bool {
        self.active.is_some() || self.queued.is_some()
    }
    pub fn clear(&mut self) {
        self.queued = None;
        if let Some(active) = &self.active {
            active.cancel.cancel();
        }
    }
}
impl<K: Send + 'static, T: Send + 'static> Worker<K, T> {
    pub fn request(
        &mut self,
        key: K,
        work: impl FnOnce(Cancel) -> anyhow::Result<T> + Send + 'static,
    ) {
        self.clear();
        self.queued = Some(Job {
            key,
            work: Box::new(work),
        });
        self.start();
    }
    fn start(&mut self) {
        if self.active.is_some() {
            return;
        }
        let Some(job) = self.queued.take() else {
            return;
        };
        let cancel = Cancel::default();
        let token = cancel.clone();
        let language = i18n::language();
        let (tx, rx) = mpsc::channel();
        let handle = thread::spawn(move || {
            let _language = i18n::using(language);
            let _ = tx.send((job.work)(token).map_err(|error| format!("{error:#}")));
        });
        self.active = Some(Active {
            key: job.key,
            cancel,
            handle,
            rx,
        });
    }
    pub fn poll(&mut self) -> Option<(K, Result<T, String>)> {
        if !self
            .active
            .as_ref()
            .is_some_and(|active| active.handle.is_finished())
        {
            return None;
        }
        let active = self.active.take().expect("completed reader");
        let result = if active.handle.join().is_err() {
            Err(i18n::tr!(BackgroundWorkerPanicked).into())
        } else {
            active
                .rx
                .try_recv()
                .unwrap_or_else(|_| Err(i18n::tr!(BackgroundWorkerTerminatedUnexpectedly).into()))
        };
        let output = (!active.cancel.is_cancelled()).then_some((active.key, result));
        self.start();
        output
    }
}
impl<K, T> Drop for Worker<K, T> {
    fn drop(&mut self) {
        self.clear();
        if let Some(active) = self.active.take() {
            // 页面返回不等待磁盘读取；任务持有取消令牌，结束后自行释放快照。
            if active.handle.is_finished() {
                let _ = active.handle.join();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};
    #[test]
    fn rapid_requests_coalesce_to_the_latest_without_publishing_cancelled_content() {
        let mut reader = Worker::default();
        let (entered_tx, entered_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        reader.request(1, move |_| {
            entered_tx.send(()).unwrap();
            release_rx.recv().unwrap();
            Ok(1)
        });
        entered_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        reader.request(2, |_| panic!("superseded queued request must not execute"));
        reader.request(3, |_| Ok(3));
        assert!(reader.poll().is_none());
        release_tx.send(()).unwrap();
        let start = Instant::now();
        loop {
            if let Some((key, result)) = reader.poll() {
                assert_eq!(key, 3);
                assert_eq!(result.unwrap(), 3);
                break;
            }
            assert!(start.elapsed() < Duration::from_secs(2));
            thread::sleep(Duration::from_millis(2));
        }
        assert!(!reader.pending());
    }
}
