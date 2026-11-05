//! Interactive target editing and explicit force-push review.
use crate::{Editor, Effect, Mode, Outcome, Page, Screen};
use crossterm::event::{KeyCode, KeyEvent};
use engine::{
    cancel::Cancel,
    operations::push::{Preparation, PushPlan, TargetRequest},
    service::Service,
};
use std::sync::Arc;

#[derive(Debug)]
enum Draft {
    Editing(Editor),
    Preparing(Editor),
}
#[derive(Debug)]
pub(crate) struct Session {
    pub request: TargetRequest,
    pub selected: usize,
    draft: Draft,
}
impl Session {
    pub fn new(request: TargetRequest) -> Self {
        let selected = request
            .remotes()
            .position(|remote| remote.name == "origin")
            .unwrap_or(0);
        let draft = Draft::Editing(Editor::new(request.branch.clone()));
        Self {
            request,
            selected,
            draft,
        }
    }
    pub fn editor(&self) -> &Editor {
        match &self.draft {
            Draft::Editing(editor) | Draft::Preparing(editor) => editor,
        }
    }
    pub fn editing(&self) -> bool {
        matches!(self.draft, Draft::Editing(_))
    }
    pub fn insert(&mut self, text: &str) {
        if let Draft::Editing(editor) = &mut self.draft {
            editor.insert(text);
        }
    }
    pub fn restore(&mut self) {
        self.draft = Draft::Editing(self.editor().clone());
    }
    pub fn choose(&mut self, selected: usize) {
        if self.editing() && selected < self.request.remotes().count() {
            self.selected = selected;
        }
    }
}

pub(crate) fn prepare_or_push(
    service: Arc<Service>,
    preparation: Preparation,
    cancel: &Cancel,
) -> anyhow::Result<Outcome> {
    match preparation {
        Preparation::Ready(plan) if !plan.needs_confirmation() => {
            Ok(Outcome::Published(service.push(&plan, cancel)?))
        }
        preparation => Ok(Outcome::PushPrepared(Box::new(preparation))),
    }
}

pub(crate) fn preparation_notice(preparation: &Preparation) -> &'static str {
    match preparation {
        Preparation::Ready(_) => i18n::tr!(PushConfirmationNeeded),
        Preparation::Target(_) => i18n::tr!(PushTargetNeeded),
        Preparation::Current => i18n::tr!(SyncCurrent),
    }
}

pub(crate) fn review(plan: &PushPlan) -> String {
    let target = plan.target();
    let detail = i18n::tf!(
        RepositoryBranchRemoteUpstreamUrlPendingCommits,
        plan.path.display(),
        target.branch,
        target.remote,
        target.remote_ref,
        plan.url(),
        target.ahead_commits
    );
    if let Some(behind) = plan.force_behind() {
        return format!(
            "{}: {} → {}/{}\n↑{} ↓{behind}\n\n{}: {behind}\n{}: {}\n{}: {}\n\n{detail}",
            i18n::tr!(Branch),
            target.branch,
            target.remote,
            target.remote_ref.trim_start_matches("refs/heads/"),
            target.ahead_commits,
            i18n::tr!(PushForceWarning),
            i18n::tr!(PushRemoteTip),
            target
                .remote_oid
                .to_string()
                .chars()
                .take(12)
                .collect::<String>(),
            i18n::tr!(PushNewTip),
            &target.local_oid[..12]
        );
    }
    detail
}

impl Screen {
    pub(crate) fn finish_push_preparation(&mut self, preparation: Preparation, origin: Mode) {
        match preparation {
            Preparation::Ready(plan) if plan.needs_confirmation() => {
                self.workspace.save(origin);
                self.mode = Mode::PushConfirm {
                    plan,
                    scroll: 0,
                    choice: crate::confirmation::Choice::Return,
                };
                self.bar = crate::bar::InputBar::CommandsOnly;
            }
            Preparation::Ready(plan) => self.execute_push(plan),
            Preparation::Target(request) => {
                self.workspace.save(origin);
                self.mode = Mode::PushTarget(Box::new(Session::new(request)));
                self.bar = crate::bar::InputBar::CommandsOnly;
            }
            Preparation::Current => {
                self.mode = origin;
                self.message = i18n::tr!(SyncCurrent).into();
            }
        }
    }
    pub(crate) fn execute_push(&mut self, plan: PushPlan) {
        let title = if plan.force_behind().is_some() {
            i18n::tr!(PushForceTitle)
        } else {
            i18n::tr!(PushCurrentBranch)
        };
        self.start_repository(
            plan.path.clone(),
            title,
            Effect::Push,
            move |service, cancel, _| Ok(Outcome::Published(service.push(&plan, &cancel)?)),
        );
    }
    pub(crate) fn cancel_push(&mut self) {
        self.mode = Mode::Browsing;
        if matches!(self.page, Page::View(_)) {
            self.return_to_view();
        }
        self.sync_bar();
    }
    pub(crate) fn push_target_key(&mut self, mut session: Box<Session>, key: KeyEvent) {
        if key.code == KeyCode::Esc {
            self.cancel_push();
            return;
        }
        if !session.editing() {
            self.mode = Mode::PushTarget(session);
            return;
        }
        match key.code {
            KeyCode::Up => session.selected = session.selected.saturating_sub(1),
            KeyCode::Down => {
                session.selected = (session.selected + 1).min(session.request.remotes().count() - 1)
            }
            KeyCode::Enter => {
                let remote = session
                    .request
                    .remotes()
                    .nth(session.selected)
                    .expect("selected remote")
                    .name
                    .clone();
                let branch = session.editor().text.clone();
                let request = session.request.clone();
                session.draft = Draft::Preparing(session.editor().clone());
                self.mode = Mode::PushTarget(session);
                self.start_repository(
                    request.path.clone(),
                    i18n::tr!(PushCurrentBranch),
                    Effect::Push,
                    move |service, cancel, _| {
                        let plan =
                            service.prepare_push_target(&request, &remote, &branch, &cancel)?;
                        prepare_or_push(service, Preparation::Ready(plan), &cancel)
                    },
                );
                return;
            }
            _ => {
                if let Draft::Editing(editor) = &mut session.draft {
                    editor.key(key);
                }
            }
        }
        self.mode = Mode::PushTarget(session);
    }
}
