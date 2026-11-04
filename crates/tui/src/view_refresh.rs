//! Local refresh targets are explicit and never delegate to remote operations.
use crate::{Configuration, Effect, Mode, Outcome, Page, Screen, history, stashes};
use anyhow::Result;
use engine::cancel::Cancel;
use std::path::PathBuf;

pub(crate) enum Target {
    Report,
    Management,
    Configuration,
    Workspace,
    Browser,
    Files(PathBuf),
    Branches(PathBuf),
    History(PathBuf),
    Stashes(PathBuf),
}
impl Target {
    pub fn current(screen: &Screen) -> Result<Self, &'static str> {
        Ok(match &screen.mode {
            Mode::Management(_) => Self::Management,
            Mode::Browser(_) => Self::Browser,
            Mode::Panel(_) => Self::Workspace,
            Mode::Files(files) => Self::Files(files.snapshot.path.clone()),
            Mode::Changes(session) => Self::Files(session.files.snapshot.path.clone()),
            Mode::Branches(view) => Self::Branches(view.session().snapshot.path.clone()),
            Mode::History(view) => match view.as_ref() {
                history::View::Graph(session) => Self::History(session.references.path.clone()),
                _ => return Err(i18n::tr!(RefreshHistorical)),
            },
            Mode::Stashes(view) => match view.as_ref() {
                stashes::View::List(session) => Self::Stashes(session.snapshot.path.clone()),
                _ => return Err(i18n::tr!(RefreshHistorical)),
            },
            Mode::Browsing => match screen.page {
                Page::Repositories => Self::Report,
                Page::Detail => Self::Workspace,
                Page::Configuration(Configuration::General | Configuration::Schedule) => {
                    Self::Configuration
                }
                _ => return Err(i18n::tr!(RefreshUnavailable)),
            },
            _ => return Err(i18n::tr!(RefreshUnavailable)),
        })
    }
    pub fn label(&self) -> &'static str {
        match self {
            Self::Report => i18n::tr!(Repositories),
            Self::Management => i18n::tr!(RepositoryManagement),
            Self::Configuration => i18n::tr!(GeneralSettings),
            Self::Workspace => i18n::tr!(CurrentRepo),
            Self::Browser => i18n::tr!(BrowserTitle),
            Self::Files(_) => i18n::tr!(ChangesTitle),
            Self::Branches(_) => i18n::tr!(BranchTitle),
            Self::History(_) => i18n::tr!(HistoryTitle),
            Self::Stashes(_) => i18n::tr!(StashTitle),
        }
    }
}
impl Screen {
    pub(crate) fn refresh_current_view(&mut self, cancel: &Cancel) {
        let target = match Target::current(self) {
            Ok(target) => target,
            Err(reason) => {
                self.message = reason.into();
                return;
            }
        };
        match target {
            Target::Report | Target::Configuration => {
                self.reload_for_refresh(cancel);
            }
            Target::Management => {
                if !self.reload_for_refresh(cancel) {
                    return;
                }
                if let Mode::Management(session) = &mut self.mode {
                    session.refresh(&self.config.repositories);
                }
            }
            Target::Workspace => {
                if !self.reload_for_refresh(cancel) {
                    return;
                }
                self.refresh_workspace();
            }
            Target::Browser => {
                if let Mode::Browser(session) = &mut self.mode {
                    session.refresh();
                }
                self.poll_browser();
            }
            Target::History(path) => self.open_history(path),
            Target::Files(path) => {
                self.refresh_overview();
                self.read_files(path);
            }
            Target::Branches(path) => {
                self.refresh_local(path, crate::mutations::readback::Requested::Branches)
            }
            Target::Stashes(path) => {
                self.refresh_local(path, crate::mutations::readback::Requested::Stashes)
            }
        }
    }
    fn reload_for_refresh(&mut self, cancel: &Cancel) -> bool {
        match self.reload(cancel) {
            Ok(()) => true,
            Err(error) => {
                self.error(i18n::tr!(RefreshFailed), error);
                false
            }
        }
    }
    fn refresh_local(&mut self, path: PathBuf, requested: crate::mutations::readback::Requested) {
        self.refresh_overview();
        self.start_repository(
            path.clone(),
            i18n::tr!(RefreshContextView),
            Effect::ReadOnly,
            move |service, cancel, _| {
                Ok(Outcome::ViewRefreshed(
                    requested.read_with_cancel(&service, &path, &cancel)?,
                ))
            },
        );
    }
}
