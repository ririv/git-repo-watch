//! Local command dispatch is private to the prepared execution module.
use crate::workspace::{Arrangement, Section, Workspace};
use crate::*;
use engine::service::ConfigEdit;
impl Screen {
    pub(super) fn activate_local(
        &mut self,
        action: Action,
        path: Option<PathBuf>,
        selection: crate::clipboard::Selection,
        return_to: crate::bar::ReturnTo,
    ) -> Result<bool> {
        match action {
            Action::OpenEditor
            | Action::Terminal
            | Action::CopyPath
            | Action::Copy
            | Action::Replay(_)
            | Action::Refresh
            | Action::Stage
            | Action::Unstage
            | Action::Discard
            | Action::DiscardFile
            | Action::LogMore
            | Action::LogParent
            | Action::BrowserDiff
            | Action::BrowserLayout(_)
            | Action::BrowserMode(_)
            | Action::StashPreview(_)
            | Action::ConflictAccept(_)
            | Action::ConflictUndo
            | Action::ConflictReset => {
                unreachable!("resolved target owns this command execution")
            }
            Action::LogSelect => self.mark_history(history::SelectionAction::ToggleMode),
            Action::LogSelectAll => self.mark_history(history::SelectionAction::All),
            Action::LogSelectClear => self.mark_history(history::SelectionAction::Clear),
            Action::Compose(action) => self.compose_action(action, path),
            Action::Commit if matches!(self.mode, Mode::CommitMessage { .. }) => {
                if let Mode::CommitMessage { editor, .. } = &mut self.mode {
                    editor.focus(crate::commit_form::Focus::Input);
                }
            }
            Action::CommitGenerate if matches!(self.mode, Mode::CommitMessage { .. }) => {
                self.generate_commit_message()
            }
            Action::ConflictAcceptMenu => self.open_conflict_controls(),
            Action::ConflictBlockMenu => {
                self.open_option_menu(command::options::Kind::ConflictBlock, path, selection)
            }
            Action::ConflictBlock(direction) => self.navigate_conflict(direction),
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
            Action::SearchMenu => self.search_menu(),
            Action::Search(mode) => self.set_search_mode(mode),
            Action::Tasks => self.open_tasks(),
            Action::Focus => self.enter_focus(),
            Action::Zoom => self.zoom_pane(),
            Action::BrowseFiles => {
                if let Some(path) = path {
                    self.open_view(crate::navigation::View::Files, path);
                } else {
                    self.message =
                        crate::message::Message::error(i18n::tr!(SelectARepositoryFirst).into());
                }
            }
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
            Action::StashPreviewMenu => {
                self.open_option_menu(crate::command::options::Kind::StashPreview, path, selection)
            }
            Action::StashSave => {
                self.open_option_menu(command::options::Kind::StashSave, path, selection)
            }
            Action::StashSaveMode(scope) => self.prepare_stash_save(scope, path, selection),
            Action::Stashes => {
                if let Some(path) = path {
                    self.open_view(crate::navigation::View::Stashes, path);
                } else {
                    self.message =
                        crate::message::Message::error(i18n::tr!(SelectARepositoryFirst).into());
                }
            }
            Action::Fetch => {
                if let Some(path) = path {
                    self.fetch_repository(path);
                } else {
                    self.message =
                        crate::message::Message::error(i18n::tr!(SelectARepositoryFirst).into());
                }
            }
            Action::ReplayControl(control) => {
                let mut draft = self.mutation_draft(action, path);
                draft.return_to = return_to;
                self.prepare_mutation(
                    engine::operations::mutation::Request::ReplayControl(control),
                    draft,
                );
            }
            Action::Pull => {
                let mut draft = self.mutation_draft(action, path);
                draft.return_to = return_to;
                self.prepare_mutation(engine::operations::mutation::Request::Pull, draft);
            }
            Action::Sync => {
                let mut draft = self.mutation_draft(action, path);
                draft.return_to = return_to;
                self.prepare_mutation(engine::operations::mutation::Request::Sync, draft);
            }
            Action::FilterMenu => self.list_menu(repository_list::Menu::Filter),
            Action::SortMenu => self.list_menu(repository_list::Menu::Sort),
            Action::Filter(filter) => self.filter_repositories(filter),
            Action::Sort(order) => self.sort_repositories(order),
            Action::Branches => {
                if let Some(path) = path {
                    self.open_view(crate::navigation::View::Branches, path);
                } else {
                    self.message =
                        crate::message::Message::error(i18n::tr!(SelectARepositoryFirst).into());
                }
            }
            Action::Quit => return Ok(true),
            Action::OpenRepository => {
                if let Some(path) = path {
                    self.open_repository(path);
                } else {
                    self.message =
                        crate::message::Message::error(i18n::tr!(SelectARepositoryFirst).into());
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
                    self.message =
                        crate::message::Message::error(i18n::tr!(SelectARepositoryFirst).into());
                }
            }
            Action::DiffLayoutMenu => {
                self.open_option_menu(command::options::Kind::DiffLayout, path, selection);
            }
            Action::PartialSelectMenu => {
                self.open_option_menu(command::options::Kind::DiffSelection, path, selection);
            }
            Action::PartialSelect(mode) => self.select_partial(mode),
            Action::Diff => {
                if let Some(path) = path {
                    self.open_view(crate::navigation::View::Diff, path);
                } else {
                    self.message =
                        crate::message::Message::error(i18n::tr!(SelectARepositoryFirst).into());
                }
            }
            Action::DiffGroup(group) => {
                if let Some(path) = path {
                    self.open_changes_group(path, group);
                } else {
                    self.message =
                        crate::message::Message::error(i18n::tr!(SelectARepositoryFirst).into());
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
                    self.message =
                        crate::message::Message::error(i18n::tr!(SelectARepositoryFirst).into())
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
                    self.message =
                        crate::message::Message::error(i18n::tr!(SelectARepositoryFirst).into())
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
    fn zoom_pane(&mut self) {
        let arrangement = if let Some(inspect) = self.mode.inspection_mut() {
            Some(&mut inspect.arrangement)
        } else {
            match &mut self.mode {
                Mode::Changes(session) => Some(&mut session.arrangement),
                Mode::Browser(session) => Some(&mut session.arrangement),
                mode if self.page == Page::Detail
                    && Workspace::is_root(mode)
                    && Section::focused(mode).is_some() =>
                {
                    self.workspace
                        .session_mut()
                        .map(|session| &mut session.arrangement)
                }
                _ => None,
            }
        };
        if let Some(arrangement) = arrangement {
            *arrangement = match arrangement {
                Arrangement::Tiled => Arrangement::Maximized,
                Arrangement::Maximized => Arrangement::Tiled,
            };
        } else {
            self.message = crate::message::Message::error(i18n::tr!(PaneUnavailable).into());
        }
    }
}
