//! Reviewed repository mutations use the same scoped background queue as index writes.
pub(crate) mod readback;
use crate::{Action, Effect, Mode, Outcome, Screen, bar::CommandDraft, command::ArgumentAction};
use anyhow::Result;
use engine::{
    local::{branches::edit, stash},
    operations::{
        mutation::{Plan, Request},
        pull,
    },
};

impl ArgumentAction {
    pub(crate) fn mutation(self, value: &str) -> Result<Option<Request>> {
        Ok(Some(match self {
            Self::BranchCreate | Self::BranchSwitch => Request::Branch {
                action: if self == Self::BranchCreate {
                    edit::Action::Create
                } else {
                    edit::Action::Switch
                },
                name: value.into(),
            },
            Self::StashSave | Self::StashSaveAll => Request::Stash(stash::Request::Save {
                mode: if self == Self::StashSave {
                    stash::SaveMode::Tracked
                } else {
                    stash::SaveMode::IncludeUntracked
                },
                message: value.into(),
            }),
            Self::StashApply => Request::Stash(stash::Request::Apply(value.parse()?)),
            Self::StashPop => Request::Stash(stash::Request::Pop(value.parse()?)),
            Self::StashDrop => Request::Stash(stash::Request::Drop(value.parse()?)),
            _ => return Ok(None),
        }))
    }
}
pub(crate) fn title(plan: &Plan) -> &'static str {
    match plan {
        Plan::Branch(edit::Plan::Create { .. }) => i18n::tr!(BranchCreateCommand),
        Plan::Branch(edit::Plan::Switch { .. }) => i18n::tr!(BranchSwitchCommand),
        Plan::Replay(plan) => plan.title(),
        Plan::Pull(_) => i18n::tr!(PullCommand),
        Plan::Sync(_) => i18n::tr!(SyncCommand),
        Plan::Stash(stash::Plan::Save {
            mode: stash::SaveMode::Tracked,
            ..
        }) => i18n::tr!(StashSaveCommand),
        Plan::Stash(stash::Plan::Save {
            mode: stash::SaveMode::IncludeUntracked,
            ..
        }) => i18n::tr!(StashSaveAllCommand),
        Plan::Stash(stash::Plan::Restore {
            action: stash::Restore::Apply,
            ..
        }) => i18n::tr!(StashApplyCommand),
        Plan::Stash(stash::Plan::Restore {
            action: stash::Restore::Pop,
            ..
        }) => i18n::tr!(StashPopCommand),
        Plan::Stash(stash::Plan::Drop { .. }) => i18n::tr!(StashDropCommand),
    }
}
pub(crate) fn review(plan: &Plan) -> String {
    let detail = match plan {
        Plan::Branch(p) => format!(
            "{}: {}\n{}: {}",
            i18n::tr!(Branch),
            p.name(),
            i18n::tr!(MutationTarget),
            p.oid()
        ),
        Plan::Replay(plan) => crate::replay::review(plan),
        Plan::Pull(p) => match p.update {
            pull::Update::Current => i18n::tr!(PullCurrent).into(),
            pull::Update::FastForward { commits } => {
                i18n::tf!(PullReview, commits, p.oid, p.upstream())
            }
        },
        Plan::Sync(plan) => {
            use engine::operations::sync::Plan;
            match plan {
                Plan::Current { .. } => i18n::tr!(SyncCurrent).into(),
                Plan::Unavailable { reason, .. } => reason.label().into(),
                Plan::Pull(plan) => {
                    let commits = match plan.update {
                        pull::Update::Current => 0,
                        pull::Update::FastForward { commits } => commits,
                    };
                    i18n::tf!(
                        SyncPullReview,
                        branch = plan.branch,
                        upstream = plan.upstream().trim_start_matches("refs/remotes/"),
                        commits = commits,
                        oid = plan.oid
                    )
                }
                Plan::Push(plan) => i18n::tf!(
                    SyncPushReview,
                    branch = plan.target.branch,
                    remote = plan.target.remote,
                    reference = plan.target.remote_ref.trim_start_matches("refs/heads/"),
                    commits = plan.target.ahead_commits,
                    oid = plan.target.local_oid
                ),
            }
        }
        Plan::Stash(stash::Plan::Save { message, .. }) => message.clone(),
        Plan::Stash(stash::Plan::Restore { entry, .. }) => {
            format!("{}\n{}", entry.oid, entry.message)
        }
        Plan::Stash(stash::Plan::Drop { entry, .. }) => format!(
            "{}\n{}\n\n{}",
            entry.oid,
            entry.message,
            i18n::tr!(StashDropWarning)
        ),
    };
    format!(
        "{}\n\n{}: {}\n\n{}",
        title(plan),
        i18n::tr!(Path),
        plan.path().display(),
        detail
    )
}
impl Screen {
    pub(crate) fn prepare_mutation(&mut self, request: Request, draft: CommandDraft) {
        let Some(path) = draft.path.clone() else {
            self.message = i18n::tr!(SelectARepositoryFirst).into();
            return;
        };
        let effect = if matches!(request, Request::Pull | Request::Sync) {
            Effect::RepositoryWrite
        } else {
            Effect::ReadOnly
        };
        let title = if matches!(request, Request::Sync) {
            i18n::tr!(SyncPreparing)
        } else {
            i18n::tr!(ConfirmOperation)
        };
        self.start_repository(
            path.clone(),
            title,
            effect,
            move |service, cancel, progress| {
                let plan = if matches!(request, Request::Sync) {
                    service.prepare_mutation_observed(&path, request, &cancel, &mut |stage| {
                        progress.enter(stage)
                    })?
                } else {
                    service.prepare_mutation(&path, request, &cancel)?
                };
                Ok(Outcome::MutationPrepared {
                    plan: Box::new(plan),
                    draft,
                })
            },
        );
    }
    pub(crate) fn execute_mutation(&mut self, plan: Plan, origin: Mode) {
        self.mode = origin;
        self.sync_bar();
        let mut spec = crate::task_center::spec(
            title(&plan),
            Some(plan.path().into()),
            engine::tasks::Retry::Unavailable,
        );
        if let Plan::Replay(replay) = &plan {
            spec.operation = Some(replay.operation());
        }
        self.start_repository_tracked(
            plan.path().into(),
            title(&plan),
            Effect::RepositoryWrite,
            spec,
            move |service, cancel, _| {
                Ok(Outcome::MutationCompleted(service.mutate(&plan, &cancel)?))
            },
        );
    }
    pub(crate) fn fetch_repository(&mut self, path: std::path::PathBuf) {
        self.fetch_repository_retry(path, None);
    }
    pub(crate) fn fetch_repository_retry(
        &mut self,
        path: std::path::PathBuf,
        parent: Option<String>,
    ) {
        let mut spec = crate::task_center::spec(
            i18n::tr!(FetchCommand),
            Some(path.clone()),
            engine::tasks::Retry::Fetch { path: path.clone() },
        );
        spec.retry_of = parent;
        self.start_repository_tracked(
            path.clone(),
            i18n::tr!(FetchCommand),
            Effect::RepositoryWrite,
            spec,
            move |service, cancel, _| {
                Ok(Outcome::MutationCompleted(service.fetch(&path, &cancel)?))
            },
        );
    }
    pub(crate) fn refresh_mutated_view(&mut self) {
        self.message = i18n::tr!(MutationComplete).into();
        if let Mode::History(view) = &self.mode {
            self.open_history(view.session().references.path.clone());
        }
    }
    pub(crate) fn mutation_draft(
        &self,
        action: Action,
        path: Option<std::path::PathBuf>,
    ) -> CommandDraft {
        let command = crate::command::COMMANDS
            .iter()
            .find(|c| c.action == action)
            .expect("catalog action");
        CommandDraft {
            input: crate::command::Input::from_canonical(
                None,
                crate::Editor::new(format!("/{}", command.name)),
            ),
            path,
            selection: self.copy_selection(),
            return_to: crate::bar::ReturnTo::Command,
        }
    }
}
