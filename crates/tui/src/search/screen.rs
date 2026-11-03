//! Bind supported search operations to the currently displayed list.
use super::{Matches, Mode as SearchMode, Support};
use crate::{Mode, Page, Screen, bar::InputBar, branches, history, stashes};
use crossterm::event::{KeyCode, KeyEvent};

impl Screen {
    pub(crate) fn search_support(&self) -> Support {
        if self.tasks.session.open() {
            return Support::Unavailable;
        }
        match &self.mode {
            Mode::Management(_) => Support::Filter,
            Mode::Browsing if self.page == Page::Repositories => Support::Both(self.search_mode),
            Mode::Browser(session) if session.focus == crate::browser::Focus::Tree => {
                Support::Filter
            }
            Mode::Browser(session) if session.content_searchable() => Support::Locate,
            Mode::Files(files) => Support::Both(files.search_mode),
            Mode::Changes(session) if session.focus == crate::changes::Focus::Files => {
                Support::Both(session.files.search_mode)
            }
            Mode::Branches(view) => match view.as_ref() {
                branches::View::List(session) => Support::Both(session.search_mode),
                branches::View::Detail { .. } => Support::Unavailable,
            },
            Mode::Stashes(view) => match view.as_ref() {
                stashes::View::List(session) => Support::Both(session.search_mode),
                stashes::View::Detail { .. } => Support::Unavailable,
            },
            Mode::History(view) => match view.as_ref() {
                history::View::Graph(_) => Support::Locate,
                history::View::Commit(commit) if commit.focus == history::Focus::Files => {
                    Support::Both(commit.search_mode)
                }
                history::View::Commit(_) | history::View::Diff { .. } => Support::Unavailable,
            },
            _ => Support::Unavailable,
        }
    }
    pub(crate) fn search_commands(&self) -> Option<Vec<&'static crate::command::Command>> {
        let support = self.search_support();
        self.bar.commands(self.config.ai.enabled).map(|commands| {
            commands
                .into_iter()
                .filter(|command| support.admits(command.action))
                .collect()
        })
    }
    pub(crate) fn search_menu(&mut self) {
        if self.search_support().mode().is_none() {
            self.message = i18n::tr!(SearchUnavailable).into();
            return;
        }
        self.open_option_menu(
            crate::command::options::Kind::Search,
            self.scope_path(),
            self.copy_selection(),
        );
    }
    pub(crate) fn set_search_mode(&mut self, mode: SearchMode) {
        match &mut self.mode {
            Mode::Browsing if self.page == Page::Repositories => {
                let path = self.selected_path();
                self.search_mode = mode;
                self.focus(path);
                if mode == SearchMode::Locate
                    && let Some(index) = self
                        .repository_matches()
                        .nearest(self.repository_viewport.selected())
                {
                    self.select(index);
                }
            }
            Mode::Files(files) => files.set_search_mode(mode),
            Mode::Changes(session) => {
                session.files.set_search_mode(mode);
                session.focus = crate::changes::Focus::Files;
            }
            Mode::Branches(view) => {
                if let branches::View::List(session) = view.as_mut() {
                    session.set_search_mode(mode);
                }
            }
            Mode::Stashes(view) => {
                if let stashes::View::List(session) = view.as_mut() {
                    session.set_search_mode(mode);
                }
            }
            Mode::History(view) => {
                if let history::View::Commit(commit) = view.as_mut() {
                    commit.set_search_mode(mode);
                }
            }
            _ => {}
        }
        self.sync_bar();
        self.poll_changes();
    }
    pub(crate) fn repository_matches(&self) -> Matches {
        let query = self.query.to_lowercase();
        Matches::new(
            self.rows()
                .iter()
                .map(|row| row.path.to_string_lossy().to_lowercase().contains(&query)),
        )
    }
    pub(crate) fn locate_key(&mut self, key: KeyEvent) -> bool {
        if !matches!(self.bar, InputBar::Search(_))
            || self.bar.editor().text.is_empty()
            || self.search_support().mode() != Some(SearchMode::Locate)
            || matches!(&self.mode, Mode::Changes(session) if session.focus == crate::changes::Focus::Preview)
            || !key.modifiers.is_empty()
            || !matches!(key.code, KeyCode::Up | KeyCode::Down)
        {
            return false;
        }
        let backwards = key.code == KeyCode::Up;
        match &mut self.mode {
            Mode::Browser(session) if session.focus == crate::browser::Focus::Preview => {
                session.next_content_match(backwards)
            }
            Mode::Browsing => {
                if let Some(index) = self
                    .repository_matches()
                    .step(self.repository_viewport.selected(), backwards)
                {
                    self.select(index);
                }
            }
            Mode::Files(files) => {
                if let Some(index) = files.matches().step(files.cursor.index(), backwards) {
                    files.locate(index);
                }
            }
            Mode::Changes(session) => {
                let files = &mut session.files;
                if let Some(index) = files.matches().step(files.cursor.index(), backwards) {
                    files.locate(index);
                }
                session.focus = crate::changes::Focus::Files;
            }
            Mode::Branches(view) => {
                if let branches::View::List(session) = view.as_mut()
                    && let Some(index) = session.matches().step(session.state.selected(), backwards)
                {
                    session.state.select(Some(index));
                }
            }
            Mode::Stashes(view) => {
                if let stashes::View::List(session) = view.as_mut()
                    && let Some(index) = session.matches().step(session.state.selected(), backwards)
                {
                    session.state.select(Some(index));
                }
            }
            Mode::History(view) => match view.as_mut() {
                history::View::Graph(session) => session.next_match(backwards),
                history::View::Commit(commit) => {
                    if let Some(index) = commit.matches().step(commit.files.selected(), backwards) {
                        commit.files.select(Some(index));
                    }
                }
                history::View::Diff { .. } => {}
            },
            _ => {}
        }
        self.poll_changes();
        true
    }
    pub(crate) fn search_title(&self) -> String {
        let Some(mode) = self.search_support().mode() else {
            return i18n::tr!(CommandsAlt).into();
        };
        let target = match &self.mode {
            Mode::Browser(session) if session.focus == crate::browser::Focus::Preview => {
                i18n::tr!(SearchTargetContent)
            }
            Mode::Browsing | Mode::Management(_) => i18n::tr!(SearchTargetRepositories),
            Mode::Browser(_) | Mode::Files(_) | Mode::Changes(_) => i18n::tr!(SearchTargetFiles),
            Mode::Branches(_) => i18n::tr!(SearchTargetBranches),
            Mode::Stashes(_) => i18n::tr!(SearchTargetStashes),
            Mode::History(view) if matches!(view.as_ref(), history::View::Graph(_)) => {
                i18n::tr!(SearchTargetCommits)
            }
            Mode::History(_) => i18n::tr!(SearchTargetFiles),
            _ => unreachable!("searchable view has a target"),
        };
        let mut title = i18n::tf!(SearchInputTitle, mode = mode.label(), target = target);
        if mode == SearchMode::Locate && !self.bar.editor().text.is_empty() {
            if let Mode::Browser(session) = &self.mode {
                title.push_str(&format!(" · {}", session.content_match_label()));
                return title;
            }
            let (matches, current) = match &self.mode {
                Mode::Browsing => (
                    self.repository_matches(),
                    self.repository_viewport.selected(),
                ),
                Mode::Files(files) => (files.matches(), files.cursor.index()),
                Mode::Changes(session) => (session.files.matches(), session.files.cursor.index()),
                Mode::Branches(view) => (view.session().matches(), view.session().state.selected()),
                Mode::Stashes(view) => (view.session().matches(), view.session().state.selected()),
                Mode::History(view) => match view.as_ref() {
                    history::View::Graph(session) => (session.matches(), session.state.selected()),
                    history::View::Commit(commit) => (commit.matches(), commit.files.selected()),
                    history::View::Diff { .. } => unreachable!(),
                },
                _ => unreachable!(),
            };
            title.push_str(&format!(" · {}", matches.label(current)));
        }
        title
    }
}
