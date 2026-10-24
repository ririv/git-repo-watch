//! A complete replacement prompt or an explicit request for the built-in default.
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case", deny_unknown_fields)]
pub enum Prompt {
    #[default]
    Default,
    Custom {
        text: String,
    },
}
impl Prompt {
    pub fn text<'a>(&'a self, default: &'a str) -> &'a str {
        match self {
            Self::Default => default,
            Self::Custom { text } => text,
        }
    }
    pub fn validate(&self) -> Result<()> {
        if let Self::Custom { text } = self {
            ensure!(
                !text.trim().is_empty() && text.len() <= 16384 && !text.contains('\0'),
                i18n::tr!(AgentPromptInvalid)
            );
        }
        Ok(())
    }
}
