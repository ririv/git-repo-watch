//! Bounded group scheduling with serialized progress delivery.
use crate::cancel::Cancel;
use anyhow::{Result, ensure};
use std::{
    collections::VecDeque,
    path::PathBuf,
    sync::{Mutex, mpsc},
    thread,
};
#[derive(Debug)]
pub enum Event<T> {
    Started { index: usize, path: PathBuf },
    Finished { index: usize, value: T },
}
pub fn run<T: Send>(
    groups: Vec<Vec<(usize, PathBuf)>>,
    limit: usize,
    cancel: &Cancel,
    task: impl Fn(&std::path::Path) -> T + Sync,
    mut emit: impl FnMut(Event<T>),
) -> Result<()> {
    ensure!((1..=32).contains(&limit), "并发数必须在 1 至 32 之间");
    let count = groups.len().min(limit);
    let queue = Mutex::new(VecDeque::from(groups));
    let (tx, rx) = mpsc::channel();
    thread::scope(|scope| -> Result<()> {
        let mut handles = Vec::new();
        for _ in 0..count {
            let tx = tx.clone();
            let queue = &queue;
            let task = &task;
            handles.push(scope.spawn(move || -> Result<()> {
                loop {
                    cancel.check()?;
                    let group = queue
                        .lock()
                        .map_err(|_| anyhow::anyhow!("任务队列锁异常"))?
                        .pop_front();
                    let Some(group) = group else { break };
                    for (index, path) in group {
                        cancel.check()?;
                        tx.send(Event::Started {
                            index,
                            path: path.clone(),
                        })
                        .map_err(|_| anyhow::anyhow!("进度接收端已关闭"))?;
                        let value = task(&path);
                        tx.send(Event::Finished { index, value })
                            .map_err(|_| anyhow::anyhow!("结果接收端已关闭"))?;
                    }
                }
                Ok(())
            }));
        }
        drop(tx);
        for event in rx {
            emit(event)
        }
        for handle in handles {
            handle
                .join()
                .map_err(|_| anyhow::anyhow!("检查工作线程异常"))??;
        }
        Ok(())
    })?;
    cancel.check()
}
