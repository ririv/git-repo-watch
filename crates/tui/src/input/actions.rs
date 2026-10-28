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
        selection: crate::clipboard::Selection,
        root: &Cancel,
    ) -> Result<bool> {
        if !action.available(self.config.ai.enabled) || !self.search_support().admits(action) {
            self.message = i18n::tr!(NoMatchingCommands).into();
            return Ok(false);
        }
        if !matches!(
            action,
            Action::Commit | Action::CommitGenerate | Action::Tasks
        ) && let Mode::CommitMessage { editor, .. } = &mut self.mode
        {
            self.commit_generator.cancel();
            editor.restore();
        }
        match action {
            Action::Compose(action) => self.compose_action(action, path),
            Action::Commit if matches!(self.mode, Mode::CommitMessage { .. }) => {
                if let Mode::CommitMessage { editor, .. } = &mut self.mode {
                    editor.focus(crate::commit_form::Focus::Message);
                }
            }
            Action::CommitGenerate if matches!(self.mode, Mode::CommitMessage { .. }) => {
                self.generate_commit_message()
            }
            Action::SearchMenu => self.search_menu(),
            Action::Search(mode) => self.set_search_mode(mode),
            Action::Tasks => self.open_tasks(),
            Action::Focus => self.enter_focus(),
            Action::Zoom => self.zoom_pane(),
            Action::PanLeft | Action::PanRight => self.pan_graph(action == Action::PanLeft),
            Action::BrowseFiles => {
                if let Some(path) = path {
                    self.open_view(crate::navigation::View::Files, path);
                } else {
                    self.message = i18n::tr!(SelectARepositoryFirst).into();
                }
            }
            Action::BrowserDiff => self.open_browser_diff(),
            Action::BrowserToggle
            | Action::BrowserSplit
            | Action::BrowserSource
            | Action::BrowserPreview => {
                if let Mode::Browser(session) = &mut self.mode
                    && let Some(target) = selection.opener()
                {
                    match action {
                        Action::BrowserToggle => {
                            session.present(target, target.presentation.toggle())
                        }
                        Action::BrowserSplit => {
                            session.present(target, target.presentation.split())
                        }
                        Action::BrowserSource => {
                            session.open_with(target, crate::browser::opening::TextMode::Source)
                        }
                        Action::BrowserPreview => {
                            session.open_with(target, crate::browser::opening::TextMode::Preview)
                        }
                        _ => unreachable!(),
                    }
                    self.poll_browser();
                }
            }
            Action::BrowserRefresh => {
                if let Mode::Browser(session) = &mut self.mode {
                    session.refresh();
                    self.poll_browser();
                }
            }
            Action::Stashes => {
                if let Some(path) = path {
                    self.open_view(crate::navigation::View::Stashes, path);
                } else {
                    self.message = i18n::tr!(SelectARepositoryFirst).into();
                }
            }
            Action::Fetch => {
                if let Some(path) = path {
                    self.fetch_repository(path);
                } else {
                    self.message = i18n::tr!(SelectARepositoryFirst).into();
                }
            }
            Action::Pull => {
                let draft = self.mutation_draft(action, path);
                self.prepare_mutation(engine::operations::mutation::Request::Pull, draft);
            }
            Action::Sync => {
                let draft = self.mutation_draft(action, path);
                self.prepare_mutation(engine::operations::mutation::Request::Sync, draft);
            }
            Action::Copy(scope) => self.copy_to_clipboard(scope, selection, path),
            Action::FilterMenu => self.list_menu(repository_list::Menu::Filter),
            Action::SortMenu => self.list_menu(repository_list::Menu::Sort),
            Action::Filter(filter) => self.filter_repositories(filter),
            Action::Sort(order) => self.sort_repositories(order),
            Action::Branches => {
                if let Some(path) = path {
                    self.open_view(crate::navigation::View::Branches, path);
                } else {
                    self.message = i18n::tr!(SelectARepositoryFirst).into();
                }
            }
            Action::Quit => return Ok(true),
            Action::OpenRepository => {
                if let Some(path) = path {
                    self.open_repository(path);
                } else {
                    self.message = i18n::tr!(SelectARepositoryFirst).into();
                }
            }
            Action::Repositories => self.open_repository_list(path),
            Action::RepositorySettings => {
                self.focus(path);
                self.open_configuration(Configuration::Repositories);
            }
            Action::Settings => self.open_configuration(Configuration::General),
            Action::AiSettings => self.open_ai_setting(crate::settings::Item::AiEnabled),
            Action::Schedule => self.open_configuration(Configuration::Schedule),
            Action::Help => self.open_page(Page::Help),
            Action::WrapMenu => {
                self.open_option_menu(command::options::Kind::TextWrap, path, selection);
            }
            Action::Wrap(enabled) => {
                self.start(
                    i18n::tr!(TextWrap),
                    Effect::Configuration,
                    move |service, _, _| {
                        service.configure(ConfigEdit::TextWrap(enabled))?;
                        Ok(Outcome::TextWrapConfigured(enabled))
                    },
                );
            }
            Action::InteractionMenu => self.interaction_menu(),
            Action::Interaction(mode) => self.set_interaction(mode),
            Action::RepositoryRefresh => {
                if self.repository_list_active() {
                    if let Err(error) = self.reload(root) {
                        self.error(i18n::tr!(RefreshFailed), error);
                    }
                } else {
                    self.message = i18n::tr!(RepositoryListOnly).into();
                }
            }
            Action::Refresh => {
                if let Mode::Browser(session) = &mut self.mode {
                    session.refresh();
                    self.poll_browser();
                    return Ok(false);
                }
                if self.page == Page::Detail && crate::workspace::Workspace::is_root(&self.mode) {
                    if let Err(error) = self.reload(root) {
                        self.error(i18n::tr!(RefreshFailed), error);
                        return Ok(false);
                    }
                    self.refresh_workspace();
                    if let Mode::Branches(view) = &self.mode {
                        self.read_branches(view.session().snapshot.path.clone());
                    } else if let Mode::History(view) = &self.mode {
                        self.open_history(view.session().references.path.clone());
                    } else if let Some(session) = self.mode.file_session() {
                        self.read_files(session.snapshot.path.clone());
                    }
                    return Ok(false);
                }
                self.refresh_overview();
                if let Mode::Stashes(view) = &self.mode {
                    self.read_stashes(view.session().snapshot.path.clone());
                } else if let Mode::Branches(view) = &self.mode {
                    self.read_branches(view.session().snapshot.path.clone());
                } else if let Mode::History(view) = &self.mode {
                    self.open_history(view.session().references.path.clone());
                } else if let Some(session) = self.mode.file_session() {
                    let path = session.snapshot.path.clone();
                    self.read_files(path);
                } else if let Err(e) = self.reload(root) {
                    self.error(i18n::tr!(RefreshFailed), e)
                }
            }
            Action::FileLayoutMenu => {
                self.open_option_menu(command::options::Kind::FileView, path, selection);
            }
            Action::FileLayout(layout) => {
                match &mut self.mode {
                    Mode::Files(files) => files.set_layout(layout),
                    Mode::Changes(session) => session.files.set_layout(layout),
                    _ => {}
                }
                self.file_layout = layout;
            }
            Action::DiffLayout(preference) => {
                self.diff_preference = preference;
                self.message = i18n::tf!(DiffLayoutSelected, preference.label());
            }
            Action::Log => {
                if let Some(path) = path {
                    self.open_view(crate::navigation::View::History, path);
                } else {
                    self.message = i18n::tr!(SelectARepositoryFirst).into();
                }
            }
            Action::LogMore => self.more_history(),
            Action::DiffLayoutMenu => {
                self.open_layout_picker(
                    crate::bar::CommandDraft {
                        input: command::Input::global(Editor::new("/repo diff layout".into())),
                        path,
                        selection,
                    },
                    0,
                );
            }
            Action::Stage | Action::Unstage => {
                self.edit_files(if action == Action::Stage {
                    IndexAction::Stage
                } else {
                    IndexAction::Unstage
                });
            }
            Action::Diff => {
                if let Some(path) = path {
                    self.open_view(crate::navigation::View::Diff, path);
                } else {
                    self.message = i18n::tr!(SelectARepositoryFirst).into();
                }
            }
            Action::Commit | Action::CommitGenerate | Action::Push => {
                if let Some(path) = path {
                    if matches!(action, Action::Commit | Action::CommitGenerate) {
                        self.start_repository(
                            path.clone(),
                            i18n::tr!(ReadStagedContents),
                            Effect::ReadOnly,
                            move |s, c, _| {
                                let plan = s.prepare_commit(&path, &c)?;
                                Ok(if action == Action::CommitGenerate {
                                    Outcome::CommitForGeneration(plan)
                                } else {
                                    Outcome::CommitPrepared(plan)
                                })
                            },
                        )
                    } else {
                        self.start_repository(
                            path.clone(),
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
                    self.start_repository(
                        path.clone(),
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
            Action::CancelCheck => {
                if !self.cancel_repository_task() {
                    self.cancel_check();
                }
            }
            Action::Workflow(trigger) => self.open_workflow(trigger),
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
            Action::AiPrompt => {
                self.open_ai_setting(crate::settings::Item::Command(Action::AiPrompt));
                self.mode = Mode::PromptEditor(Box::new(crate::prompt_editor::Session::new(
                    &self.config.ai.prompt,
                )));
                self.sync_bar();
            }
            _ => {
                let value = match action {
                    Action::Interval => {
                        engine::scheduler::duration::compact(self.config.interval_seconds)
                    }
                    Action::Anchor => self
                        .config
                        .schedule_anchor
                        .basis()
                        .map(|basis| basis.at.format("%Y-%m-%d %H:%M:%S").to_string())
                        .unwrap_or_else(|| "auto".into()),
                    Action::Concurrency => self.config.max_concurrency.to_string(),
                    Action::AiModel => {
                        if self.config.ai.program().model.is_empty() {
                            "default".into()
                        } else {
                            self.config.ai.program().model.clone()
                        }
                    }
                    _ => String::new(),
                };
                let argument = action.argument().expect("parameter action");
                self.bar = crate::bar::InputBar::Command {
                    input: command::Input::from_canonical(
                        self.command_context(),
                        Editor::new(format!("{}{value}", argument.command().completion())),
                    ),
                    selected: 0,
                    menu: Default::default(),
                    path,
                    selection: self.copy_selection(),
                };
                self.refresh_option_candidates();
            }
        }
        Ok(false)
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
        self.open_page(Page::Configuration(section));
    }
    fn open_ai_setting(&mut self, item: crate::settings::Item) {
        self.open_configuration(Configuration::General);
        self.setting_selected = Configuration::General
            .items(self.snapshot.registration, &self.config.ai)
            .iter()
            .position(|candidate| *candidate == item)
            .expect("enabled AI setting is present in the general form");
        self.settings_focus = None;
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
        self.mode = Mode::Changes(Box::new(crate::changes::Session::new(
            files,
            crate::changes::ReturnTo::Files,
        )));
        self.poll_changes();
        self.sync_bar();
    }
    pub(crate) fn edit_files(&mut self, action: IndexAction) {
        let Some(session) = self.mode.file_session().cloned() else {
            self.message = i18n::tr!(ChangesSelectionRequired).into();
            return;
        };
        let selected = session.selection();
        if selected.is_empty() {
            self.message = i18n::tr!(NoActionableFilesInThisArea).into();
            return;
        }
        if selected
            .iter()
            .any(|change| (change.area == Area::Staged) != (action == IndexAction::Unstage))
        {
            self.message = i18n::tr!(ChangesWrongArea).into();
            return;
        }
        self.start_repository(
            session.snapshot.path.clone(),
            if action == IndexAction::Stage {
                i18n::tr!(StageSelectedFiles)
            } else {
                i18n::tr!(UnstageSelectedFiles)
            },
            Effect::IndexWrite,
            move |service, cancel, _| {
                Ok(Outcome::Files(service.edit_index(
                    &session.snapshot,
                    action,
                    &selected,
                    &cancel,
                )?))
            },
        );
    }
    pub(crate) fn confirm(&mut self, confirmation: Confirmation) {
        match confirmation {
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
                                i18n::tf!(FoundRepositoriesScanSaved, report.repositories.len())
                            }
                            Parameter::Exclude => save(ConfigEdit::Exclude(value.into()))?,
                            Parameter::Unexclude => save(ConfigEdit::Unexclude(value.into()))?,
                            Parameter::Interval => {
                                save(ConfigEdit::Interval(state::interval(&value)?))?
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
                            Parameter::StartupCheck => {
                                s.configure(ConfigEdit::StartupCheck(value.parse()?))?;
                                i18n::tr!(StartupCheckSavedNextLaunch).into()
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
