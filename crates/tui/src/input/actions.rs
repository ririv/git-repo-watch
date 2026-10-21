//! Explicit command workflows over application services.
use super::super::*;
use anyhow::bail;
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
        match action {
            Action::Quit => return Ok(true),
            Action::Settings => {
                self.page = Page::Settings;
                self.mode = Mode::Browsing;
                self.scroll = 0
            }
            Action::Help => {
                self.history.push(self.page);
                self.page = Page::Help;
                self.mode = Mode::Browsing;
                self.scroll = 0
            }
            Action::Refresh => {
                if let Some(session) = self.mode.file_session() {
                    let path = session.snapshot.path.clone();
                    let area = session.area;
                    self.read_files(path, area);
                } else if let Err(e) = self.reload(root) {
                    self.error(i18n::tr!(RefreshFailed), e)
                }
            }
            Action::Stage | Action::Unstage | Action::Diff => {
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
                    self.mode = Mode::Confirm(Confirmation::Remove(path))
                } else {
                    self.message = i18n::tr!(SelectARepositoryFirst).into()
                }
            }
            Action::CheckAll => self.mode = Mode::Confirm(Confirmation::CheckAll),
            Action::Apply => self.mode = Mode::Confirm(Confirmation::Apply),
            _ => {
                let value = match action {
                    Action::Interval => format!("{}s", self.config.interval_seconds),
                    Action::Autopush => self.config.push_policy.to_string(),
                    Action::Concurrency => self.config.max_concurrency.to_string(),
                    _ => String::new(),
                };
                self.mode = Mode::Input {
                    action,
                    editor: Editor::new(value),
                }
            }
        }
        Ok(false)
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
                format!("{}\n\n{content}", change.location.display()),
            ))
        })
    }
    pub(crate) fn edit_files(&mut self) {
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
            Confirmation::CheckAll => self.start(
                i18n::tr!(CheckAllRepositories),
                Effect::Check,
                move |s, c, tx| {
                    let report = s.run(&c, |event| {
                        let _ = tx.send(WorkerEvent::Progress(event));
                    })?;
                    Ok(Outcome::Checked(report))
                },
            ),
            Confirmation::Apply => self.start(
                i18n::tr!(ApplySystemSchedule),
                Effect::Configuration,
                move |s, c, _| {
                    s.install(&c)?;
                    Ok(Outcome::Text(
                        i18n::tr!(SystemScheduleAppliedNoCheckStarted).into(),
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
            Confirmation::Configure { action, value } => self.start(
                command::description(action),
                Effect::Configuration,
                move |s, c, _| {
                    let message = match action {
                        Action::Add => {
                            i18n::tf!(AddedRepositories, s.add(&[PathBuf::from(value)])?)
                        }
                        Action::Scan | Action::Import => {
                            let report = s.scan(
                                Path::new(&value),
                                if action == Action::Scan {
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
                        _ => {
                            let edit = match action {
                                Action::Exclude => ConfigEdit::Exclude(value.into()),
                                Action::Unexclude => ConfigEdit::Unexclude(value.into()),
                                Action::Interval => ConfigEdit::Interval(state::interval(&value)?),
                                Action::Autopush => ConfigEdit::Policy(value.parse()?),
                                Action::Concurrency => ConfigEdit::Concurrency(value.parse()?),
                                _ => {
                                    bail!(i18n::tr!(ThisCommandDoesNotAcceptConfigurationInput))
                                }
                            };
                            s.configure(edit)?;
                            i18n::tr!(ConfigurationSavedApplyScheduleAfterChangingThe).into()
                        }
                    };
                    Ok(Outcome::Text(message))
                },
            ),
        }
    }
}
