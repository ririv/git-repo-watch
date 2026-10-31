//! Serialize field edits and finish pending choices before the screen is dropped.
use anyhow::Result;
use engine::{
    cancel::Cancel,
    service::{ConfigEdit, Service},
    state::view::Edit,
};
use std::{
    collections::VecDeque,
    sync::Arc,
    thread::{self, JoinHandle},
};

#[derive(Default)]
pub(crate) struct Writer {
    state: State,
}
#[derive(Default)]
enum State {
    #[default]
    Idle,
    Saving {
        service: Arc<Service>,
        edit: Edit,
        queued: VecDeque<Edit>,
        handle: JoinHandle<Result<()>>,
    },
}
impl Writer {
    pub fn submit(&mut self, service: Arc<Service>, edit: Edit) {
        match &mut self.state {
            State::Idle => self.begin(service, edit, VecDeque::new()),
            State::Saving { queued, .. } => {
                // 每个字段只保留最新意图；字段之间的写入互不覆盖。
                queued.retain(|pending| !pending.same_field(edit));
                queued.push_back(edit);
            }
        }
    }
    fn begin(&mut self, service: Arc<Service>, edit: Edit, queued: VecDeque<Edit>) {
        let worker = service.clone();
        let language = i18n::language();
        let handle = thread::spawn(move || {
            let _language = i18n::using(language);
            worker.configure_with_cancel(ConfigEdit::View(edit), &Cancel::default())
        });
        self.state = State::Saving {
            service,
            edit,
            queued,
            handle,
        };
    }
    #[cfg(test)]
    pub fn pending(&self) -> bool {
        matches!(self.state, State::Saving { .. })
    }
    pub fn poll(&mut self) -> Option<(Edit, Result<()>)> {
        if !matches!(&self.state, State::Saving { handle, .. } if handle.is_finished()) {
            return None;
        }
        self.collect()
    }
    fn collect(&mut self) -> Option<(Edit, Result<()>)> {
        let State::Saving {
            service,
            edit,
            mut queued,
            handle,
        } = std::mem::take(&mut self.state)
        else {
            return None;
        };
        let result = handle
            .join()
            .unwrap_or_else(|_| Err(anyhow::anyhow!(i18n::tr!(BackgroundWorkerPanicked))));
        if let Some(next) = queued.pop_front() {
            self.begin(service, next, queued);
        }
        Some((edit, result))
    }
    pub fn finish(&mut self) -> Result<()> {
        let mut failure = None;
        while let Some((_, result)) = self.collect() {
            if let Err(error) = result {
                failure.get_or_insert(error);
            }
        }
        match failure {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }
}
impl Drop for Writer {
    fn drop(&mut self) {
        if let Err(error) = self.finish() {
            eprintln!("{}: {error:#}", i18n::tr!(ViewPreferenceSaveFailed));
        }
    }
}
