//! Draft-scoped background generation, independent of repository mutation workers.
use crate::{Effect, Mode, Outcome, Screen, spinner};
use engine::{cancel::Cancel, commit_message, local::review::CommitPlan};
use std::{
    thread::{self, JoinHandle},
    time::Instant,
};

#[derive(Debug)]
struct Request {
    draft_id: u64,
    plan: CommitPlan,
    updates: std::sync::mpsc::Receiver<commit_message::Update>,
    cancel: Cancel,
    animation: spinner::Spinner,
    handle: JoinHandle<anyhow::Result<String>>,
}
#[cfg(test)]
type Response = fn(&Cancel, &mut dyn FnMut(commit_message::Update)) -> anyhow::Result<String>;
#[derive(Debug, Default)]
pub(crate) struct Generator {
    request: Option<Request>,
    #[cfg(test)]
    pub response: Option<Response>,
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
    pub(crate) fn commit_form_action(&mut self, action: crate::commit_form::Action) {
        use crate::commit_form::{Action, EditAction};
        match action {
            Action::CancelGeneration => self.commit_generator.cancel(),
            Action::Edit(EditAction::Regenerate) => self.generate_commit_message(),
            Action::Edit(EditAction::Commit) => self.submit_commit(),
            Action::Edit(EditAction::Return) => {
                self.take_mode();
                if matches!(self.page, crate::Page::View(_)) {
                    self.return_to_view();
                }
                self.message.clear();
                self.sync_bar();
            }
        }
    }
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
        if editor.submitting() {
            return;
        }
        let plan = plan.clone();
        let task_plan = plan.clone();
        let config = self.config.ai.clone();
        let cancel = Cancel::default();
        let task_cancel = cancel.clone();
        let language = i18n::language();
        let store = self.service.store.clone();
        let (updates_tx, updates) = std::sync::mpsc::channel();
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
            let emit = |update| {
                let _ = updates_tx.send(update);
            };
            #[cfg(test)]
            let result = match response {
                Some(response) => {
                    let mut emit = emit;
                    response(&task_cancel, &mut emit)
                }
                None => commit_message::generate(&task_plan, &config, &task_cancel, emit),
            };
            #[cfg(not(test))]
            let result = commit_message::generate(&task_plan, &config, &task_cancel, emit);
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
            updates,
            cancel,
            animation: spinner::Spinner::new(Instant::now()),
            handle,
        });
        if let Mode::CommitMessage { editor, .. } = &mut self.mode {
            editor.begin();
        }
        self.sync_bar();
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
        let mut changed = false;
        if let Some(request) = &self.commit_generator.request
            && request.draft_id == self.commit_draft_id
            && !request.cancel.is_cancelled()
            && let Mode::CommitMessage { plan, editor, .. } = &mut self.mode
            && *plan == request.plan
        {
            for update in request.updates.try_iter() {
                editor.update(update);
                changed = true;
            }
        }
        if !self
            .commit_generator
            .request
            .as_ref()
            .is_some_and(|request| request.handle.is_finished())
        {
            return changed;
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
            if request.draft_id == self.commit_draft_id
                && let Mode::CommitMessage { plan, editor, .. } = &mut self.mode
                && *plan == request.plan
            {
                editor.restore();
            }
            return true;
        }
        let Mode::CommitMessage { plan, editor, .. } = &mut self.mode else {
            return true;
        };
        if *plan != request.plan {
            return true;
        }
        if !editor.generating() {
            return true;
        }
        if request.cancel.is_cancelled() {
            editor.restore();
            self.message = crate::message::Message::information(i18n::tr!(AgentCancelled).into());
        } else {
            match result {
                Ok(message) => {
                    editor.accept(message);
                    self.message =
                        crate::message::Message::information(i18n::tr!(AgentGenerated).into());
                }
                Err(error) => {
                    editor.restore();
                    self.error(i18n::tr!(AgentGenerating), error);
                }
            }
        }
        true
    }
    pub(crate) fn submit_commit(&mut self) {
        let Mode::CommitMessage { plan, editor, .. } = &self.mode else {
            return;
        };
        if editor.generating() || editor.submitting() {
            return;
        }
        if editor.text().trim().is_empty() {
            self.message = crate::message::Message::error(i18n::tr!(EnterACommitMessage).into());
            return;
        }
        let plan = plan.clone();
        let message = editor.text().to_owned();
        if self.repository_tasks.contains_key(&plan.path) {
            self.message = crate::message::Message::error(i18n::tr!(RepositoryTaskBusy).into());
            return;
        }
        if self.repository_tasks.len() >= self.config.max_concurrency {
            self.message = crate::message::Message::error(i18n::tr!(RepositoryTaskLimit).into());
            return;
        }
        self.start_repository(
            plan.path.clone(),
            i18n::tr!(CommitStagedContents),
            Effect::Commit,
            move |service, cancel, _| {
                Ok(Outcome::Committed(
                    service.commit(&plan, &message, &cancel)?,
                ))
            },
        );
        if let Mode::CommitMessage { editor, .. } = &mut self.mode {
            editor.submit();
        }
        self.sync_bar();
    }
}
