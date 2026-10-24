//! Draft-scoped background generation, independent of repository mutation workers.
use crate::{Editor, Mode, Screen, spinner};
use engine::{cancel::Cancel, commit_message, local::review::CommitPlan};
use std::{
    thread::{self, JoinHandle},
    time::Instant,
};

#[derive(Debug)]
struct Request {
    draft_id: u64,
    plan: CommitPlan,
    original: String,
    cancel: Cancel,
    animation: spinner::Spinner,
    handle: JoinHandle<anyhow::Result<String>>,
}
#[derive(Debug, Default)]
pub(crate) struct Generator {
    request: Option<Request>,
    #[cfg(test)]
    pub response: Option<fn(&Cancel) -> anyhow::Result<String>>,
}
impl Generator {
    pub fn running(&self) -> bool {
        self.request.is_some()
    }
    pub fn label(&self) -> Option<String> {
        self.request.as_ref().map(|request| {
            format!(
                "{} {}",
                request.animation.symbol(),
                if request.cancel.is_cancelled() {
                    i18n::tr!(Cancelling)
                } else {
                    i18n::tr!(AgentGenerating)
                }
            )
        })
    }
    pub fn tick(&mut self) -> bool {
        self.request
            .as_mut()
            .is_some_and(|request| request.animation.tick(Instant::now()))
    }
    pub fn cancel(&self) {
        if let Some(request) = &self.request {
            request.cancel.cancel();
        }
    }
    pub fn shutdown(&mut self) {
        if let Some(request) = self.request.take() {
            request.cancel.cancel();
            let _ = request.handle.join();
        }
    }
}
impl Screen {
    pub(crate) fn generate_commit_message(&mut self) {
        if !self.config.ai.enabled {
            return;
        }
        if self.commit_generator.running() {
            self.commit_generator.cancel();
            return;
        }
        let Mode::CommitMessage { plan, editor, .. } = &self.mode else {
            return;
        };
        let plan = plan.clone();
        let task_plan = plan.clone();
        let config = self.config.ai.clone();
        let cancel = Cancel::default();
        let task_cancel = cancel.clone();
        let language = i18n::language();
        let store = self.service.store.clone();
        #[cfg(test)]
        let response = self.commit_generator.response;
        let handle = thread::spawn(move || {
            let _language = i18n::using(language);
            let run = engine::tasks::Run::begin(
                &store,
                crate::task_center::spec(
                    i18n::tr!(AgentGenerating),
                    Some(task_plan.path.clone()),
                    engine::tasks::Retry::Unavailable,
                ),
                &task_cancel,
            )?;
            #[cfg(test)]
            let result = match response {
                Some(response) => response(&task_cancel),
                None => commit_message::generate(&task_plan, &config, &task_cancel),
            };
            #[cfg(not(test))]
            let result = commit_message::generate(&task_plan, &config, &task_cancel);
            run.finish(engine::tasks::Outcome::capture(
                &result,
                &task_cancel,
                Clone::clone,
            ))?;
            result
        });
        self.commit_generator.request = Some(Request {
            draft_id: self.commit_draft_id,
            plan,
            original: editor.text.clone(),
            cancel,
            animation: spinner::Spinner::new(Instant::now()),
            handle,
        });
        self.message.clear();
    }
    pub(crate) fn poll_commit_message(&mut self) -> bool {
        if let Some(request) = &self.commit_generator.request
            && (!self.config.ai.enabled
                || request.draft_id != self.commit_draft_id
                || !matches!(&self.mode, Mode::CommitMessage { plan, .. } if *plan == request.plan))
        {
            request.cancel.cancel();
        }
        if !self
            .commit_generator
            .request
            .as_ref()
            .is_some_and(|request| request.handle.is_finished())
        {
            return false;
        }
        let request = self
            .commit_generator
            .request
            .take()
            .expect("completed generation");
        let result = request
            .handle
            .join()
            .unwrap_or_else(|_| Err(anyhow::anyhow!(i18n::tr!(BackgroundWorkerPanicked))));
        if !self.config.ai.enabled || request.draft_id != self.commit_draft_id {
            return true;
        }
        let Mode::CommitMessage { plan, editor, .. } = &mut self.mode else {
            return true;
        };
        if *plan != request.plan {
            return true;
        }
        if request.cancel.is_cancelled() {
            self.message = i18n::tr!(AgentCancelled).into();
        } else if editor.text != request.original {
            // 请求只拥有启动时的草稿，用户后续输入永远优先。
            self.message = i18n::tr!(AgentDraftChanged).into();
        } else {
            match result {
                Ok(message) => {
                    *editor = Editor::new(message);
                    self.message = i18n::tr!(AgentGenerated).into();
                }
                Err(error) => self.error(i18n::tr!(AgentGenerating), error),
            }
        }
        true
    }
}
