//! Explicit feedback kinds keep error styling independent from translated text.
use std::{fmt, ops::Deref};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Message {
    Information(String),
    Error(String),
}
impl Message {
    pub fn information(text: String) -> Self {
        Self::Information(text)
    }
    pub fn error(text: String) -> Self {
        Self::Error(text)
    }
    pub fn as_str(&self) -> &str {
        match self {
            Self::Information(text) | Self::Error(text) => text,
        }
    }
    pub fn is_error(&self) -> bool {
        matches!(self, Self::Error(_))
    }
    pub fn clear(&mut self) {
        *self = Self::default();
    }
    pub fn append_error(&mut self, text: &str) {
        *self = Self::Error(format!("{}{text}", self.as_str()));
    }
}
impl Default for Message {
    fn default() -> Self {
        Self::Information(String::new())
    }
}
impl From<String> for Message {
    fn from(text: String) -> Self {
        Self::Information(text)
    }
}
impl From<&str> for Message {
    fn from(text: &str) -> Self {
        Self::Information(text.into())
    }
}
impl Deref for Message {
    type Target = str;
    fn deref(&self) -> &str {
        self.as_str()
    }
}
impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
impl PartialEq<str> for Message {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}
impl PartialEq<&str> for Message {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}
impl PartialEq<String> for Message {
    fn eq(&self, other: &String) -> bool {
        self.as_str() == other
    }
}
impl PartialEq<Message> for str {
    fn eq(&self, other: &Message) -> bool {
        self == other.as_str()
    }
}
impl PartialEq<Message> for &str {
    fn eq(&self, other: &Message) -> bool {
        *self == other.as_str()
    }
}
impl PartialEq<Message> for String {
    fn eq(&self, other: &Message) -> bool {
        self == other.as_str()
    }
}
