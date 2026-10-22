//! One worker and a bounded queue; evicted tiles are cancelled before encoding.
use super::{Backend, Key};
use anyhow::Result;
use engine::cancel::Cancel;
use image::RgbaImage;
use ratatui_image::protocol::Protocol;
use std::{
    collections::VecDeque,
    sync::{Arc, Condvar, Mutex, mpsc},
    thread,
};
pub(super) struct Job {
    pub key: Key,
    pub id: u32,
    pub cancel: Cancel,
    pub backend: Backend,
    pub language: i18n::Language,
    pub prepare: Box<dyn FnOnce() -> Result<RgbaImage> + Send>,
}
pub(super) struct Response {
    pub key: Key,
    pub id: u32,
    pub result: Result<Protocol>,
}
enum Queue {
    Open(VecDeque<Job>),
    Closed,
}
pub(super) struct Worker {
    inbox: Arc<(Mutex<Queue>, Condvar)>,
    channel: Option<(mpsc::Receiver<Response>, thread::JoinHandle<()>)>,
}
impl Worker {
    pub fn new() -> Self {
        Self {
            inbox: Arc::new((Mutex::new(Queue::Open(VecDeque::new())), Condvar::new())),
            channel: None,
        }
    }
    pub fn push(&mut self, job: Job) {
        if self.channel.is_none() {
            let inbox = self.inbox.clone();
            let (tx, rx) = mpsc::sync_channel(super::CAPACITY);
            let handle = thread::spawn(move || run(inbox, tx));
            self.channel = Some((rx, handle));
        }
        let mut state = self.inbox.0.lock().expect("image worker queue");
        if let Queue::Open(queue) = &mut *state {
            queue.retain(|job| !job.cancel.is_cancelled());
            // 缓存最多保留 CAPACITY 个任务，删除已淘汰任务后队列不会超过此上限。
            assert!(queue.len() < super::CAPACITY);
            queue.push_back(job);
            self.inbox.1.notify_one();
        }
    }
    pub fn responses(&self) -> Result<Vec<Response>> {
        let Some((rx, _)) = &self.channel else {
            return Ok(Vec::new());
        };
        let mut responses = Vec::new();
        loop {
            match rx.try_recv() {
                Ok(response) => responses.push(response),
                Err(mpsc::TryRecvError::Empty) => return Ok(responses),
                Err(mpsc::TryRecvError::Disconnected) => {
                    anyhow::bail!(i18n::tr!(GraphicsWorkerStopped))
                }
            }
        }
    }
}
fn run(inbox: Arc<(Mutex<Queue>, Condvar)>, tx: mpsc::SyncSender<Response>) {
    loop {
        let job = {
            let mut state = inbox.0.lock().expect("image worker queue");
            loop {
                match &mut *state {
                    Queue::Closed => return,
                    Queue::Open(queue) => {
                        if let Some(job) = queue.pop_front() {
                            break job;
                        }
                    }
                }
                state = inbox.1.wait(state).expect("image worker queue");
            }
        };
        if job.cancel.is_cancelled() {
            continue;
        }
        let _language = i18n::using(job.language);
        let result = (job.prepare)().and_then(|image| {
            job.cancel.check()?;
            job.backend.prepare(image, job.id)
        });
        if job.cancel.is_cancelled() {
            continue;
        }
        if tx
            .send(Response {
                key: job.key,
                id: job.id,
                result,
            })
            .is_err()
        {
            return;
        }
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        *self.inbox.0.lock().expect("image worker queue") = Queue::Closed;
        self.inbox.1.notify_one();
        if let Some((rx, handle)) = self.channel.take() {
            // 先关闭接收端，避免工作线程在已满的结果队列上阻塞退出。
            drop(rx);
            let _ = handle.join();
        }
    }
}
