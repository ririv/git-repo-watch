//! Streaming text generation shared by independent AI capabilities.
mod claude;
mod codex;
pub(crate) mod runner;
pub(crate) mod wire;
use super::{Agent, Config};
use crate::cancel::Cancel;
use anyhow::{Result, ensure};
pub const MESSAGE_LIMIT: usize = 64 * 1024;
#[derive(Debug, Clone)]
pub enum Update {
    Append(String),
    Replace(String),
}
pub fn generate(
    config: &Config,
    prompt: &str,
    cancel: &Cancel,
    mut emit: impl FnMut(Update) + Send,
) -> Result<String> {
    ensure!(config.enabled, i18n::tr!(AiDisabled));
    config.validate()?;
    let executable = runner::resolve(config.agent)?;
    runner::run(config, prompt, cancel, &executable, &mut emit)
}
