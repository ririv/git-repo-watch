//! Explicit reasons why a repository operation does not apply.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkipReason {
    NoRemotes,
}
impl SkipReason {
    pub fn label(self) -> &'static str {
        match self {
            Self::NoRemotes => i18n::tr!(NoRemoteConfiguredSkipped),
        }
    }
}
