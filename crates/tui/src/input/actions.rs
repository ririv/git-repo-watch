//! Explicit command workflows over application services.
use super::super::*;
use engine::{
    local::files::IndexAction,
    service::{ConfigEdit, ScanMode},
    state,
};
use std::path::Path;
impl Screen {
    pub(crate) fn activate(
        &mut self,
        action: Action,
        path: Option<PathBuf>,
        root: &Cancel,
    ) -> Result<bool> {
        if self.check_task.is_some()
            && matches!(
                action,
                Action::Commit
                    | Action::Push
                    | Action::Add
                    | Action::Remove
                    | Action::Exclude
                    | Action::Unexclude
                    | Action::Scan
                    | Action::Import
                    | Action::Interval
                    | Action::Autopush
                    | Action::Concurrency
                    | Action::StartupCheck
                    | Action::Apply
                    | Action::EnableSchedule
                    | Action::DisableSchedule
            )
        {
            self.message = i18n::tr!(CheckWriteConflict).into();
            return Ok(false);
        }
        match action {
            Action::Quit => return Ok(true),
            Action::Repositories => {
                self.focus(path);
                self.open_configuration(Configuration::Repositories);
            }
            Action::Settings => self.open_configuration(Configuration::General),
            Action::Schedule => self.open_configuration(Configuration::Schedule),
            Action::Help => {
                if self.page == Page::Detail {
                    self.park_workspace();
                }
                self.history.push(self.page);
                self.page = Page::Help;
                self.mode = Mode::Browsing;
                self.scroll = 0
            }
            Action::Refresh => {
                if self.page == Page::Detail && crate::workspace::Workspace::is_root(&self.mode) {
                    if let Err(error) = self.reload(root) {
                        self.error(i18n::tr!(RefreshFailed), error);
                        return Ok(false);
                    }
                    self.refresh_workspace();
                    if let Mode::History(view) = &self.mode {
                        self.open_history(view.session().references.path.clone());
                    } else if let Some(session) = self.mode.file_session() {
                        self.read_files(session.snapshot.path.clone(), session.area);
                    }
                    return Ok(false);
                }
                self.refresh_overview();
                if let Mode::History(view) = &self.mode {
                    self.open_history(view.session().references.path.clone());
                } else if let Some(session) = self.mode.file_session() {
                    let path = session.snapshot.path.clone();
                    let area = session.area;
                    self.read_files(path, area);
                } else if let Err(e) = self.reload(root) {
                    self.error(i18n::tr!(RefreshFailed), e)
                }
            }
            Action::DiffLayout(preference) => {
                self.diff_preference = preference;
                self.message = i18n::tf!(DiffLayoutSelected, preference.label());
            }
            Action::Log => {
                if let Some(path) = path {
                    self.open_history(path);
                } else {
                    self.message = i18n::tr!(SelectARepositoryFirst).into();
                }
            }
            Action::LogMore => self.more_history(),
            Action::Stage | Action::Unstage | Action::Diff => {
                if action == Action::Diff && matches!(self.mode, Mode::History(_)) {
                    self.show_history_diff();
                    return Ok(false);
                }
                if let Some(session) = self.mode.file_session() {
                    if action == Action::Diff {
                        self.show_diff();
                        return Ok(false);
                    }
                    if (action == Action::Stage && session.area == FileArea::Worktree)
                        || (action == Action::Unstage && session.area == FileArea::Index)
                    {
                        self.edit_files();
                        return Ok(false);
                    }
                }
                if let Some(path) = path {
                    self.read_files(
                        path,
                        if action == Action::Unstage {
                            FileArea::Index
                        } else {
                            FileArea::Worktree
                        },
                    )
                } else {
                    self.message = i18n::tr!(SelectARepositoryFirst).into()
                }
            }
            Action::Commit | Action::Push => {
                if let Some(path) = path {
                    if action == Action::Commit {
                        self.start(
                            i18n::tr!(ReadStagedContents),
                            Effect::ReadOnly,
                            move |s, c, _| {
                                Ok(Outcome::CommitPrepared(s.prepare_commit(&path, &c)?))
                            },
                        )
                    } else {
                        self.start(
                            i18n::tr!(ValidateCurrentBranchAndRemoteUpstream),
                            Effect::ReadOnly,
                            move |s, c, _| Ok(Outcome::PushPrepared(s.prepare_push(&path, &c)?)),
                        )
                    }
                } else {
                    self.message = i18n::tr!(SelectARepositoryFirst).into()
                }
            }
            Action::Terminal => {
                if let Some(path) = path {
                    self.start(
                        i18n::tr!(OpenRepositoryTerminal),
                        Effect::ReadOnly,
                        move |service, cancel, _| {
                            service.open_terminal(&path, &cancel)?;
                            Ok(Outcome::TerminalOpened(path))
                        },
                    );
                } else {
                    self.message = i18n::tr!(SelectARepositoryFirst).into();
                }
            }
            Action::Remove => {
                if let Some(path) = path {
                    self.mode = Mode::Confirm {
                        operation: Confirmation::Remove(path),
                        choice: Default::default(),
                    }
                } else {
                    self.message = i18n::tr!(SelectARepositoryFirst).into()
                }
            }
            Action::Check | Action::CheckAll => {
                if self.check_task.is_some() {
                    self.message = i18n::tr!(CheckAlreadyRunning).into();
                } else {
                    let target = if action == Action::CheckAll {
                        Some(checks::CheckTarget::All)
                    } else {
                        path.map(checks::CheckTarget::Repository)
                    };
                    if let Some(target) = target {
                        self.start_check(target);
                    } else {
                        self.message = i18n::tr!(SelectARepositoryFirst).into();
                    }
                }
            }
            Action::CancelCheck => self.cancel_check(),
            Action::Apply | Action::EnableSchedule | Action::DisableSchedule => {
                self.mode = Mode::Confirm {
                    operation: Confirmation::Schedule(match action {
                        Action::EnableSchedule => engine::scheduler::Control::Enable,
                        Action::DisableSchedule => engine::scheduler::Control::Disable,
                        Action::Apply => engine::scheduler::Control::Apply,
                        _ => unreachable!(),
                    }),
                    choice: Default::default(),
                }
            }
            _ => {
                let value = match action {
                    Action::Interval => format!("{}s", self.config.interval_seconds),
                    Action::Autopush => self.config.push_policy.to_string(),
                    Action::Concurrency => self.config.max_concurrency.to_string(),
                    Action::StartupCheck => self.config.check_on_startup.to_string(),
                    _ => String::new(),
                };
                let argument = action.argument().expect("parameter action");
                self.bar = crate::bar::InputBar::Command {
                    editor: Editor::new(format!("{}{value}", argument.command().completion())),
                    selected: 0,
                    menu: Default::default(),
                    path,
                };
            }
        }
        Ok(false)
    }
    pub(crate) fn prepare_configuration(
        &mut self,
        command: command::ArgumentAction,
        value: String,
        draft: crate::bar::CommandDraft,
    ) {
        if self.check_task.is_some() {
            self.message = i18n::tr!(CheckWriteConflict).into();
            return;
        }
        let origin = Box::new(self.take_mode());
        self.mode = Mode::Confirm {
            operation: Confirmation::Configure {
                command,
                value,
                origin,
                draft,
            },
            choice: Default::default(),
        };
        self.message.clear();
    }
    fn open_configuration(&mut self, section: Configuration) {
        if self.page == Page::Detail {
            self.park_workspace();
        }
        if self.page != Page::Configuration(section) {
            self.history.push(self.page);
        }
        self.page = Page::Configuration(section);
        self.setting_selected = 0;
        self.mode = Mode::Browsing;
        self.scroll = 0;
    }
    pub(crate) fn read_files(&mut self, path: PathBuf, area: FileArea) {
        self.start(
            i18n::tr!(ReadRepositoryFiles),
            Effect::ReadOnly,
            move |s, c, _| Ok(Outcome::Files(s.files(&path, &c)?, area)),
        )
    }
    pub(crate) fn show_diff(&mut self) {
        let Some(session) = self.mode.file_session().cloned() else {
            return;
        };
        let Some(change) = session.rows().get(session.cursor).map(|c| (*c).clone()) else {
            self.message = i18n::tr!(NoFilesInThisArea).into();
            return;
        };
        self.start(i18n::tr!(ReadFileDiff), Effect::ReadOnly, move |s, c, _| {
            let content = s.diff(&session.snapshot, &change, &c)?;
            Ok(Outcome::Diff(
                session,
                diff::prepare(&change.location.display(), content, change.area, &c)?,
            ))
        })
    }
    pub(crate) fn edit_files(&mut self) {
        if self.check_task.is_some() {
            self.message = i18n::tr!(CheckWriteConflict).into();
            return;
        }
        let Some(session) = self.mode.file_session().cloned() else {
            return;
        };
        let selected = session.selection();
        if selected.is_empty() {
            self.message = i18n::tr!(NoActionableFilesInThisArea).into();
            return;
        }
        let action = if session.area == FileArea::Index {
            IndexAction::Unstage
        } else {
            IndexAction::Stage
        };
        self.start(
            if action == IndexAction::Stage {
                i18n::tr!(StageSelectedFiles)
            } else {
                i18n::tr!(UnstageSelectedFiles)
            },
            Effect::IndexWrite,
            move |s, c, _| {
                Ok(Outcome::Files(
                    s.edit_index(&session.snapshot, action, &selected, &c)?,
                    session.area,
                ))
            },
        )
    }
    pub(crate) fn confirm(&mut self, confirmation: Confirmation) {
        match confirmation {
            Confirmation::Schedule(operation) => self.start(
                match operation {
                    engine::scheduler::Control::Enable => i18n::tr!(EnableScheduledTask),
                    engine::scheduler::Control::Disable => i18n::tr!(DisableScheduledTask),
                    engine::scheduler::Control::Apply => i18n::tr!(ApplySystemSchedule),
                },
                Effect::Configuration,
                move |s, c, _| {
                    s.control_schedule(operation, &c)?;
                    Ok(Outcome::Text(
                        match operation {
                            engine::scheduler::Control::Enable => i18n::tr!(ScheduleEnabledNoCheck),
                            engine::scheduler::Control::Disable => {
                                i18n::tr!(ScheduleDisabledPreserved)
                            }
                            engine::scheduler::Control::Apply => {
                                i18n::tr!(ScheduleAppliedPreserved)
                            }
                        }
                        .into(),
                    ))
                },
            ),
            Confirmation::Remove(path) => self.start(
                i18n::tr!(UnregisterRepository),
                Effect::Configuration,
                move |s, _, _| {
                    s.configure(ConfigEdit::Remove(path))?;
                    Ok(Outcome::Text(
                        i18n::tr!(RepositoryUnregisteredFilesRetained).into(),
                    ))
                },
            ),
            Confirmation::Configure {
                command,
                value,
                origin,
                ..
            } => {
                self.mode = *origin;
                self.sync_bar();
                self.start(
                    command::description(command.action()),
                    Effect::Configuration,
                    move |s, c, _| {
                        use command::ArgumentAction as Parameter;
                        let save = |edit| -> Result<String> {
                            s.configure(edit)?;
                            Ok(i18n::tr!(ConfigurationSavedApplyScheduleAfterChangingThe).into())
                        };
                        let message = match command {
                            Parameter::Add => {
                                i18n::tf!(AddedRepositories, s.add(&[PathBuf::from(value)])?)
                            }
                            Parameter::Scan | Parameter::Import => {
                                let report = s.scan(
                                    Path::new(&value),
                                    if command == Parameter::Scan {
                                        ScanMode::Preview
                                    } else {
                                        ScanMode::Import
                                    },
                                    &c,
                                )?;
                                i18n::tf!(
                                    FoundRepositoriesSeeVarImportReportJson,
                                    report.repositories.len()
                                )
                            }
                            Parameter::Exclude => save(ConfigEdit::Exclude(value.into()))?,
                            Parameter::Unexclude => save(ConfigEdit::Unexclude(value.into()))?,
                            Parameter::Interval => {
                                save(ConfigEdit::Interval(state::interval(&value)?))?
                            }
                            Parameter::Autopush => save(ConfigEdit::Policy(value.parse()?))?,
                            Parameter::Concurrency => {
                                save(ConfigEdit::Concurrency(value.parse()?))?
                            }
                            Parameter::StartupCheck => {
                                s.configure(ConfigEdit::StartupCheck(value.parse()?))?;
                                i18n::tr!(StartupCheckSavedNextLaunch).into()
                            }
                        };
                        Ok(Outcome::Text(message))
                    },
                );
            }
        }
    }
}
