//! Resolve the initial view independently from persistent repository registration.
mod setup;
use crate::{Screen, Service, clipboard, command};
use anyhow::Result;
use engine::cancel::Cancel;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Start {
    CurrentDirectory,
    Repositories,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Entry {
    Repositories,
    Repository(PathBuf),
}
impl Start {
    pub(crate) fn resolve(self, cancel: &Cancel) -> Result<Entry> {
        match self {
            Self::CurrentDirectory => self.in_directory(&std::env::current_dir()?, cancel),
            Self::Repositories => Ok(Entry::Repositories),
        }
    }
    pub(crate) fn in_directory(self, directory: &Path, cancel: &Cancel) -> Result<Entry> {
        match self {
            Self::CurrentDirectory => {
                match engine::local::containing_repository(directory, cancel)? {
                    Some(path) => Ok(Entry::Repository(path)),
                    None => Ok(Entry::Repositories),
                }
            }
            Self::Repositories => Ok(Entry::Repositories),
        }
    }
}
impl Entry {
    pub(crate) fn open(self, service: Service, cancel: &Cancel) -> Result<Screen> {
        match self {
            Self::Repositories => Screen::new(service, cancel),
            Self::Repository(path) => {
                let service = service.with_session_repository(path.clone(), cancel)?;
                let mut screen = Screen::new(service, cancel)?;
                screen.open_repository(path);
                Ok(screen)
            }
        }
    }
}

pub(super) fn open(start: Start, service: Service, cancel: &Cancel) -> Result<Option<Screen>> {
    let entry = start.resolve(cancel)?;
    if !service.store.needs_initialization()? {
        return entry.open(service, cancel).map(Some);
    }
    let Some(choice) = setup::choose(&entry, &service.store.config_path(), cancel)? else {
        return Ok(None);
    };
    cancel.check()?;
    service.store.initialize()?;
    match choice {
        setup::Choice::CurrentRepository(path) => {
            service.add(&[path.to_owned()])?;
            entry.open(service, cancel).map(Some)
        }
        setup::Choice::Directory => {
            let mut screen = Entry::Repositories.open(service, cancel)?;
            screen.open_command_entry(
                command::ConfigArgument::Add.command(),
                None,
                clipboard::Selection::None,
            );
            Ok(Some(screen))
        }
        setup::Choice::Later => entry.open(service, cancel).map(Some),
    }
}
