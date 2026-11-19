//! Explicit command workflows over application services.
use super::super::*;
use engine::{service::ConfigEdit, state};
impl Screen {
    pub(crate) fn activate(
        &mut self,
        action: Action,
        path: Option<PathBuf>,
        selection: crate::clipboard::Selection,
        root: &Cancel,
    ) -> Result<bool> {
        let execution = match self.prepare_command(action, path.as_deref(), &selection) {
            Ok(execution) => execution,
            Err(error) => {
                self.message = crate::message::Message::error(format!("{error:#}"));
                return Ok(false);
            }
        };
        if !matches!(
            action,
            Action::Commit | Action::CommitGenerate | Action::Tasks
        ) && let Mode::CommitMessage { editor, .. } = &mut self.mode
        {
            self.commit_generator.cancel();
            editor.restore();
        }
        execution.execute(self, root)
    }
    pub(crate) fn select_agent(&mut self, agent: engine::ai::Agent) {
        self.start(
            i18n::tr!(AgentChoice),
            Effect::Configuration,
            move |service, cancel, _| {
                service.configure_with_cancel(ConfigEdit::AiAgent(agent), &cancel)?;
                Ok(Outcome::Text(i18n::tr!(AgentSaved).into()))
            },
        );
    }
    pub(crate) fn prepare_configuration(
        &mut self,
        command: command::ConfigArgument,
        value: String,
        draft: crate::bar::CommandDraft,
    ) {
        if matches!(command, command::ConfigArgument::RepositoryEnvironment(_))
            && let Err(error) =
                self.prepare_command(command.action(), draft.path.as_deref(), &draft.selection)
        {
            self.message = crate::message::Message::error(format!("{error:#}"));
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
    pub(crate) fn read_files(&mut self, path: PathBuf) {
        self.start_repository(
            path.clone(),
            i18n::tr!(ReadRepositoryFiles),
            Effect::ReadOnly,
            move |service, cancel, _| Ok(Outcome::Files(service.files(&path, &cancel)?)),
        );
    }
    pub(crate) fn show_diff(&mut self) {
        let Some(files) = self.mode.file_session().cloned() else {
            return;
        };
        let group = files.group();
        self.mode = Mode::Changes(Box::new(crate::changes::Session::new(
            files,
            crate::changes::ReturnTo::Files,
            group,
        )));
        self.poll_changes();
        self.sync_bar();
    }
    pub(crate) fn confirm(&mut self, confirmation: Confirmation) {
        match confirmation {
            Confirmation::ConflictReset { plan, origin, .. } => {
                self.mode = *origin;
                self.execute_conflict_reset(*plan);
            }
            Confirmation::DiscardFile { plan, origin, .. } => {
                self.mode = *origin;
                self.execute_discard_file(plan);
            }
            Confirmation::Partial { plan, origin, .. } => {
                self.mode = *origin;
                plan.execute(self);
            }
            Confirmation::ReplayParents {
                candidate,
                selected,
                origin,
                draft,
            } => self.choose_replay_parent(*candidate, selected, *origin, draft),
            Confirmation::Mutation { plan, origin, .. } => self.execute_mutation(*plan, *origin),
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
            Confirmation::Remove { path, origin } => {
                self.mode = *origin;
                self.start(
                    i18n::tr!(UnregisterRepository),
                    Effect::Configuration,
                    move |s, _, _| {
                        s.configure(ConfigEdit::Remove(path))?;
                        Ok(Outcome::Text(
                            i18n::tr!(RepositoryUnregisteredFilesRetained).into(),
                        ))
                    },
                );
            }
            Confirmation::Configure {
                command,
                value,
                origin,
                draft,
            } => {
                let agent = self.config.ai.agent;
                self.mode = *origin;
                self.sync_bar();
                self.start(
                    command::description(command.action()),
                    Effect::Configuration,
                    move |s, c, _| {
                        use command::ConfigArgument as Parameter;
                        let save = |edit| -> Result<String> {
                            s.configure(edit)?;
                            Ok(i18n::tr!(ConfigurationSavedApplyScheduleAfterChangingThe).into())
                        };
                        let message = match command {
                            Parameter::Add => {
                                i18n::tf!(AddedRepositories, s.add(&[PathBuf::from(value)])?)
                            }
                            Parameter::Interval => {
                                save(ConfigEdit::Interval(state::interval(&value)?))?
                            }
                            Parameter::AutoFetchInterval => {
                                s.configure(ConfigEdit::AutoFetchInterval(state::interval(
                                    &value,
                                )?))?;
                                i18n::tr!(PreferenceSaved).into()
                            }
                            Parameter::Anchor => {
                                save(ConfigEdit::Anchor(engine::scheduler::timing::parse_anchor(
                                    &value,
                                    chrono::Local::now().date_naive(),
                                )?))?
                            }
                            Parameter::Autopush => save(ConfigEdit::Policy(value.parse()?))?,
                            Parameter::Concurrency => {
                                save(ConfigEdit::Concurrency(value.parse()?))?
                            }
                            Parameter::RepositoryEnvironment(field) => {
                                let path = draft.path.expect("resolved repository target");
                                s.configure_with_cancel(
                                    ConfigEdit::RepositoryEnvironment {
                                        path,
                                        edit: field.edit(&value)?,
                                    },
                                    &c,
                                )?;
                                i18n::tr!(PreferenceSaved).into()
                            }
                            Parameter::AiModel | Parameter::AiPrompt => {
                                let edit = match command {
                                    Parameter::AiModel => ConfigEdit::AiModel {
                                        agent,
                                        model: if value == "default" {
                                            String::new()
                                        } else {
                                            value
                                        },
                                    },
                                    Parameter::AiPrompt => {
                                        ConfigEdit::AiPrompt(engine::ai::Prompt::Custom {
                                            text: value,
                                        })
                                    }
                                    _ => unreachable!(),
                                };
                                s.configure_with_cancel(edit, &c)?;
                                i18n::tr!(AgentSaved).into()
                            }
                        };
                        Ok(Outcome::Text(message))
                    },
                );
            }
        }
    }
}
