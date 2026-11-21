//! External opens are bounded, cancellable and independent of repository jobs.
use anyhow::{Result, ensure};
use engine::{cancel::Cancel, desktop::Target};
use std::{
    sync::{Arc, mpsc},
    thread,
};
#[cfg(test)]
type Sink = Arc<dyn Fn(&Target, &Cancel) -> Result<()> + Send + Sync>;
struct Active {
    target: Arc<Target>,
    cancel: Cancel,
    rx: mpsc::Receiver<Result<Option<engine::desktop::Launch>, String>>,
    handle: thread::JoinHandle<()>,
}
#[derive(Default)]
pub(crate) struct Opener {
    active: Option<Active>,
    launches: Vec<(Arc<Target>, engine::desktop::Launch)>,
    #[cfg(test)]
    sink: Option<Sink>,
}
impl Opener {
    pub fn submit(&mut self, target: Arc<Target>) -> Result<()> {
        if self
            .active
            .as_ref()
            .is_some_and(|active| active.target == target)
        {
            return Ok(());
        }
        ensure!(
            self.active.is_none() && self.launches.len() < 32,
            i18n::tr!(LinkOpening)
        );
        let requested = target.clone();
        let cancel = Cancel::default();
        let token = cancel.clone();
        let (tx, rx) = mpsc::channel();
        let language = i18n::language();
        #[cfg(test)]
        let sink = self.sink.clone();
        let handle = thread::spawn(move || {
            let _language = i18n::using(language);
            #[cfg(test)]
            let result = match sink {
                Some(sink) => sink(&target, &token).map(|()| None),
                None => engine::desktop::launch(&target, &token).map(Some),
            };
            #[cfg(not(test))]
            let result = engine::desktop::launch(&target, &token).map(Some);
            let _ = tx.send(result.map_err(|error| format!("{error:#}")));
        });
        self.active = Some(Active {
            target: requested,
            cancel,
            rx,
            handle,
        });
        Ok(())
    }
    pub fn poll(&mut self) -> Option<(Arc<Target>, Result<(), String>)> {
        let mut index = 0;
        while index < self.launches.len() {
            match self.launches[index].1.poll() {
                Ok(None) => index += 1,
                Ok(Some(())) => {
                    self.launches.swap_remove(index);
                }
                Err(error) => {
                    let (target, _) = self.launches.swap_remove(index);
                    return Some((target, Err(format!("{error:#}"))));
                }
            }
        }
        let active = self.active.as_ref()?;
        if !active.handle.is_finished() {
            return None;
        }
        let active = self.active.take().expect("finished open");
        let result = if active.handle.join().is_err() {
            Err(i18n::tr!(BackgroundWorkerPanicked).into())
        } else {
            active
                .rx
                .try_recv()
                .unwrap_or_else(|_| Err(i18n::tr!(BackgroundWorkerTerminatedUnexpectedly).into()))
        };
        let target = active.target;
        let result = result.map(|launch| {
            if let Some(launch) = launch {
                self.launches.push((target.clone(), launch));
            }
        });
        Some((target, result))
    }
    pub fn shutdown(&mut self) {
        if let Some(active) = self.active.take() {
            active.cancel.cancel();
            let _ = active.handle.join();
        }
    }
    #[cfg(test)]
    pub fn with_sink(
        sink: impl Fn(&Target, &Cancel) -> Result<()> + Send + Sync + 'static,
    ) -> Self {
        Self {
            active: None,
            launches: Vec::new(),
            sink: Some(Arc::new(sink)),
        }
    }
}
impl Drop for Opener {
    fn drop(&mut self) {
        self.shutdown();
    }
}
