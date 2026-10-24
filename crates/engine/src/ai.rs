//! Shared settings for optional AI capabilities backed by local agent CLIs.
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
mod prompt;
pub mod text;
pub use prompt::Prompt;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Agent {
    #[default]
    Codex,
    Claude,
}
impl Agent {
    pub const ALL: &[Self] = &[Self::Codex, Self::Claude];
    pub fn name(self) -> &'static str {
        match self {
            Self::Codex => "codex",
            Self::Claude => "claude",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Codex => "Codex",
            Self::Claude => "Claude Code",
        }
    }
}
impl std::str::FromStr for Agent {
    type Err = anyhow::Error;
    fn from_str(name: &str) -> Result<Self> {
        Self::ALL
            .iter()
            .copied()
            .find(|agent| agent.name().eq_ignore_ascii_case(name))
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "{}: {name}; {}: {}",
                    i18n::tr!(UnknownAgent),
                    i18n::tr!(SupportedAgents),
                    Self::ALL
                        .iter()
                        .map(|agent| agent.name())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            })
    }
}
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Program {
    #[serde(default)]
    pub model: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub enabled: bool,
    pub agent: Agent,
    pub prompt: Prompt,
    pub codex: Program,
    pub claude: Program,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            enabled: true,
            agent: Agent::default(),
            prompt: Prompt::Default,
            codex: Program::default(),
            claude: Program::default(),
        }
    }
}
impl Config {
    pub fn program(&self) -> &Program {
        self.program_for(self.agent)
    }
    pub fn program_for(&self, agent: Agent) -> &Program {
        match agent {
            Agent::Codex => &self.codex,
            Agent::Claude => &self.claude,
        }
    }
    pub fn program_mut(&mut self) -> &mut Program {
        self.program_mut_for(self.agent)
    }
    pub fn program_mut_for(&mut self, agent: Agent) -> &mut Program {
        match agent {
            Agent::Codex => &mut self.codex,
            Agent::Claude => &mut self.claude,
        }
    }
    pub fn validate(&self) -> Result<()> {
        for program in [&self.codex, &self.claude] {
            ensure!(
                !program.model.chars().any(char::is_control) && program.model.len() <= 256,
                i18n::tr!(AgentModelInvalid)
            );
        }
        self.prompt.validate()
    }
}
