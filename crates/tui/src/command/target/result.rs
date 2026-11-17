//! A target result keeps readiness and its concrete value inseparable.
use std::borrow::Cow;

#[derive(Debug)]
pub(crate) enum Target<T> {
    OutOfScope(Reason),
    Unavailable(Reason),
    Ready(T),
}
#[derive(Debug, Clone)]
pub(crate) enum Reason {
    Message(Cow<'static, str>),
    WrongArea(engine::local::files::IndexAction),
}
impl Reason {
    pub fn text(&self) -> &str {
        match self {
            Self::Message(message) => message,
            Self::WrongArea(_) => i18n::tr!(ChangesWrongArea),
        }
    }
    pub fn label(&self) -> &str {
        match self {
            Self::WrongArea(engine::local::files::IndexAction::Stage) => {
                i18n::tr!(CommandNeedsUnstaged)
            }
            Self::WrongArea(engine::local::files::IndexAction::Unstage) => {
                i18n::tr!(CommandNeedsStaged)
            }
            Self::Message(message) => message,
        }
    }
}
impl From<&'static str> for Reason {
    fn from(message: &'static str) -> Self {
        Self::Message(message.into())
    }
}
impl From<String> for Reason {
    fn from(message: String) -> Self {
        Self::Message(message.into())
    }
}
impl<T> Target<T> {
    pub fn map<U>(self, map: impl FnOnce(T) -> U) -> Target<U> {
        match self {
            Self::Ready(value) => Target::Ready(map(value)),
            Self::OutOfScope(reason) => Target::OutOfScope(reason),
            Self::Unavailable(reason) => Target::Unavailable(reason),
        }
    }
    pub fn and_then<U>(self, resolve: impl FnOnce(T) -> Target<U>) -> Target<U> {
        match self {
            Self::Ready(value) => resolve(value),
            Self::OutOfScope(reason) => Target::OutOfScope(reason),
            Self::Unavailable(reason) => Target::Unavailable(reason),
        }
    }
    pub fn reason(&self) -> Option<&Reason> {
        match self {
            Self::Ready(_) => None,
            Self::OutOfScope(reason) | Self::Unavailable(reason) => Some(reason),
        }
    }
}
