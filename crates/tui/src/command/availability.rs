//! Feature visibility projects the same resolved target used by command execution.
use super::target::{CommandContext, Resolved, Target};
use crate::{Action, Screen, clipboard};
use std::path::Path;

#[derive(Debug)]
pub(crate) enum Availability<T = ()> {
    Hidden,
    Target(Target<T>),
}
impl<T> Availability<T> {
    pub fn visible(&self, discovery: super::Discovery) -> bool {
        match self {
            Self::Hidden => false,
            Self::Target(Target::OutOfScope(_)) => discovery == super::Discovery::GlobalSearch,
            Self::Target(Target::Ready(_) | Target::Unavailable(_)) => true,
        }
    }
    pub fn is_ready(&self) -> bool {
        matches!(self, Self::Target(Target::Ready(_)))
    }
    pub fn value(&self) -> Option<&T> {
        match self {
            Self::Target(Target::Ready(value)) => Some(value),
            Self::Hidden | Self::Target(Target::OutOfScope(_) | Target::Unavailable(_)) => None,
        }
    }
    pub fn reason(&self) -> Option<&str> {
        match self {
            Self::Hidden => Some(i18n::tr!(NoMatchingCommands)),
            Self::Target(target) => target.reason().map(|reason| reason.text()),
        }
    }
    pub fn label(&self) -> Option<&str> {
        match self {
            Self::Hidden => Some(i18n::tr!(NoMatchingCommands)),
            Self::Target(target) => target.reason().map(|reason| reason.label()),
        }
    }
    pub fn map<U>(self, map: impl FnOnce(T) -> U) -> Availability<U> {
        match self {
            Self::Hidden => Availability::Hidden,
            Self::Target(target) => Availability::Target(target.map(map)),
        }
    }
}
impl Screen {
    pub(crate) fn resolve_command<'a>(
        &'a self,
        action: Action,
        path: Option<&'a Path>,
        selection: &'a clipboard::Selection,
    ) -> Availability<Resolved<'a>> {
        if !action.available(&self.config)
            || (selection.is_error()
                && !matches!(action, Action::Copy | Action::Quit | Action::Tasks))
        {
            return Availability::Hidden;
        }
        let context = CommandContext::new(self, path, selection);
        Availability::Target(context.resolve(action))
    }
    pub(crate) fn prepare_command(
        &self,
        action: Action,
        path: Option<&Path>,
        selection: &clipboard::Selection,
    ) -> anyhow::Result<super::target::Execution> {
        match self.resolve_command(action, path, selection) {
            Availability::Target(Target::Ready(target)) => {
                target.capture(action, path.map(Path::to_owned), selection)
            }
            unavailable => anyhow::bail!(
                "{}",
                unavailable
                    .reason()
                    .expect("unavailable command has a reason")
            ),
        }
    }
    pub(crate) fn command_availability(
        &self,
        action: Action,
        path: Option<&Path>,
        selection: &clipboard::Selection,
    ) -> Availability {
        self.resolve_command(action, path, selection).map(|_| ())
    }
}
