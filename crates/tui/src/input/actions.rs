//! Explicit command workflows over application services.
use super::super::*;
use engine::{local::files::IndexAction, service::ConfigEdit, state};
impl Screen {
    pub(crate) fn activate(
        &mut self,
        action: Action,
        path: Option<PathBuf>,
        selection: crate::clipboard::Selection,
        root: &Cancel,
    ) -> Result<bool> {
        if !action.available(&self.config)
            || (selection.is_error()
                && !matches!(action, Action::Copy | Action::Quit | Action::Tasks))
        {
            self.message = i18n::tr!(NoMatchingCommands).into();
            return Ok(false);
        }
        if let Some(reason) = self
            .command_availability(action, path.as_deref(), &selection)
            .reason()
        {
            self.message = reason.to_owned();
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
            Action::OpenEditor => self.open_in_editor(path, &selection),
            Action::LogSelect => self.mark_history(history::SelectionAction::Start),
            Action::LogSelection(action) => self.mark_history(action),
            Action::Compose(action) => self.compose_action(action, path),
            Action::Commit if matches!(self.mode, Mode::CommitMessage { .. }) => {
                if let Mode::CommitMessage { editor, .. } = &mut self.mode {
                    editor.focus(crate::commit_form::Focus::Message);
                }
            }
            Action::CommitGenerate if matches!(self.mode, Mode::CommitMessage { .. }) => {
                self.generate_commit_message()
            }
            Action::ConflictMenu => {
                self.open_option_menu(command::options::Kind::Conflict, path, selection)
            }
            Action::ConflictView(mode) => {
                if let Mode::Changes(session) = &mut self.mode
                    && let crate::changes::Preview::Ready {
                        content, scroll, ..
                    } = &mut session.preview
                    && let crate::document::View::Conflict(view) = content.as_mut()
                {
                    view.set_mode(mode, scroll);
                }
                self.sync_bar();
            }
            Action::BranchFilterMenu => {
                self.open_option_menu(command::options::Kind::BranchFilter, path, selection)
            }
            Action::BranchFilter(filter) => {
                if let Mode::Branches(view) = &mut self.mode
                    && let branches::View::List(session) = view.as_mut()
                {
                    session.set_filter(filter);
                }
                self.sync_bar();
            }
            Action::LogParent => self.cycle_history_parent(),
            Action::SearchMenu => self.search_menu(),
            Action::Search(mode) => self.set_search_mode(mode),
            Action::Tasks => self.open_tasks(),
            Action::Focus => self.enter_focus(),
            Action::Zoom => self.zoom_pane(),
            Action::BrowseFiles => {
                if let Some(path) = path {
                    self.open_view(crate::navigation::View::Files, path);
                } else {
                    self.message = i18n::tr!(SelectARepositoryFirst).into();
                }
            }
            Action::BrowserDiff => self.open_browser_diff(),
            Action::BrowserModeMenu => {
                self.open_option_menu(command::options::Kind::FileMode, path, selection)
            }
            Action::BrowserLayoutMenu => {
                self.open_option_menu(command::options::Kind::FileLayout, path, selection)
            }
            Action::ComposeOrderMenu => {
                self.open_option_menu(command::options::Kind::ComposeOrder, path, selection)
            }
            Action::ScheduleEnabledMenu => {
                self.open_option_menu(command::options::Kind::ScheduleEnabled, path, selection)
            }
            Action::BrowserLayout(_) | Action::BrowserMode(_) => {
                if let Mode::Browser(session) = &mut self.mode
                    && let Some(target) = selection.opener()
                {
                    match action {
                        Action::BrowserLayout(layout) => {
                            session.present(target, target.presentation.with_layout(layout))
                        }
                        Action::BrowserMode(mode) => session.open_with(target, mode),
                        _ => unreachable!(),
                    }
                    self.poll_browser();
                }
            }
            Action::StashPreviewMenu => {
                self.open_option_menu(crate::command::options::Kind::StashPreview, path, selection)
            }
            Action::StashPreview(scope) => self.open_stash_detail(scope),
            Action::StashSave => {
                self.open_option_menu(command::options::Kind::StashSave, path, selection)
            }
            Action::StashSaveMode(scope) => self.prepare_stash_save(scope, path, selection),
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
            Action::Replay(kind) => {
                if let Some(commit) = selection.commit() {
                    let commits = match kind {
                        engine::operations::replay::Kind::CherryPick => selection.commits(),
                        engine::operations::replay::Kind::Revert => vec![commit],
                    };
                    let mut draft = self.mutation_draft(action, path);
                    draft.selection = selection;
                    self.prepare_replay(kind, commits, draft);
                } else {
                    self.message = i18n::tr!(ReplaySelectCommit).into();
                }
            }
            Action::ReplayControl(control) => {
                let draft = self.mutation_draft(action, path);
                self.prepare_mutation(
                    engine::operations::mutation::Request::ReplayControl(control),
                    draft,
                );
            }
            Action::Pull => {
                let draft = self.mutation_draft(action, path);
                self.prepare_mutation(engine::operations::mutation::Request::Pull, draft);
            }
            Action::Sync => {
                let draft = self.mutation_draft(action, path);
                self.prepare_mutation(engine::operations::mutation::Request::Sync, draft);
            }
            Action::CopyPath => {
                let target = selection
                    .file_target()
                    .expect("available file path has a target");
                let text = target
                    .relative_path()
                    .to_str()
                    .expect("available file path is UTF-8");
                self.submit_clipboard(Ok(crate::clipboard::Payload {
                    target: crate::clipboard::Target::FilePaths,
                    text: text.to_owned(),
                }));
            }
            Action::Copy => {
                let scope = selection.copy_scope().expect("available copy has a source");
                self.copy_to_clipboard(scope, selection, path);
            }
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
                self.open_management();
            }
            Action::Settings => self.open_configuration(Configuration::General),
            Action::AiSettings => self.open_ai_setting(crate::settings::Item::AiEnabled),
            Action::Schedule => self.open_configuration(Configuration::Schedule),
            Action::Help => self.open_page(Page::Help),
            Action::AutoFetchMenu => {
                self.open_option_menu(command::options::Kind::AutoFetch, path, selection);
            }
            Action::AutoFetch(enabled) => {
                self.start(
                    i18n::tr!(AutoFetch),
                    Effect::Configuration,
                    move |service, cancel, _| {
                        service.configure_with_cancel(
                            ConfigEdit::AutoFetchEnabled(enabled),
                            &cancel,
                        )?;
                        Ok(Outcome::Text(i18n::tr!(PreferenceSaved).into()))
                    },
                );
            }
            Action::StartupFetchMenu => {
                self.open_option_menu(command::options::Kind::StartupFetch, path, selection);
            }
            Action::StartupFetch(enabled) => {
                self.start(
                    i18n::tr!(StartupFetch),
                    Effect::Configuration,
                    move |service, _, _| {
                        service.configure(ConfigEdit::FetchOnStartup(enabled))?;
                        Ok(Outcome::Text(i18n::tr!(StartupFetchSaved).into()))
                    },
                );
            }
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
            Action::Refresh => self.refresh_current_view(root),
            Action::FileLayoutMenu => {
                self.open_option_menu(command::options::Kind::FileView, path, selection);
            }
            Action::FileLayout(layout) => {
                self.set_file_layout(layout);
            }
            Action::DiffLayout(preference) => {
                self.set_diff_layout(preference);
            }
            Action::DiffHeader => {
                self.open_option_menu(command::options::Kind::DiffHeader, path, selection)
            }
            Action::DiffHeaderVisibility(visibility) => self.set_diff_header(visibility),
            Action::DiffContextMenu => {
                self.open_option_menu(command::options::Kind::DiffContext, path, selection);
            }
            Action::DiffContext(context) => self.set_diff_context(context),
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
                        repository: self.capture_repository(path.as_deref()),
                        path,
                        selection,
                        return_to: crate::bar::ReturnTo::Command,
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
            Action::PartialSelectMenu => {
                self.open_option_menu(command::options::Kind::DiffSelection, path, selection);
            }
            Action::PartialSelect(mode) => self.select_partial(mode),
            Action::Discard => self.edit_partial(engine::local::files::partial::Action::Discard),
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
                            i18n::tr!(PushCurrentBranch),
                            Effect::Push,
                            move |s, c, _| {
                                let preparation = s.prepare_interactive_push(&path, &c)?;
                                crate::push::prepare_or_push(s, preparation, &c)
                            },
                        )
                    }
                } else {
                    self.message = i18n::tr!(SelectARepositoryFirst).into()
                }
            }
            Action::Terminal => {
                if let Some(path) = path {
                    let directory = selection
                        .terminal_target(&path)
                        .expect("available terminal has a directory");
                    self.start_repository(
                        path.clone(),
                        i18n::tr!(OpenRepositoryTerminal),
                        Effect::Terminal,
                        move |service, cancel, _| {
                            service.open_terminal(&path, &directory, &cancel)?;
                            Ok(Outcome::TerminalOpened(directory))
                        },
                    );
                } else {
                    self.message = i18n::tr!(SelectARepositoryFirst).into();
                }
            }
            Action::Remove => {
                if let Some(path) = path {
                    let origin = Box::new(self.take_mode());
                    self.mode = Mode::Confirm {
                        operation: Confirmation::Remove { path, origin },
                        choice: Default::default(),
                    }
                } else {
                    self.message = i18n::tr!(SelectARepositoryFirst).into()
                }
            }
            Action::FetchAll => self.start_check(checks::CheckTarget::All),
            Action::CancelCheck => {
                if !self.cancel_repository_task() && !self.cancel_auto_fetch() {
                    self.cancel_check();
                }
            }
            Action::Workflow => self.open_workflow(),
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
                    Action::AutoFetchInterval => engine::scheduler::duration::compact(
                        self.config.auto_fetch.interval_seconds,
                    ),
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
                    repository: self.capture_repository(path.as_deref()),
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
        use command::target::IndexSelection;
        let session = match IndexSelection::current(self) {
            Some(IndexSelection::Files(files)) => files.clone(),
            Some(IndexSelection::Diff(_)) => {
                self.edit_partial(match action {
                    IndexAction::Stage => engine::local::files::partial::Action::Stage,
                    IndexAction::Unstage => engine::local::files::partial::Action::Unstage,
                });
                return;
            }
            None => {
                self.message = i18n::tr!(ChangesSelectionRequired).into();
                return;
            }
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
            Confirmation::Partial { plan, origin, .. } => {
                self.mode = *origin;
                self.execute_partial(*plan);
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
