//! Streaming text generation shared by independent AI capabilities.
mod claude;
mod codex;
pub(crate) mod runner;
pub(crate) mod wire;
use super::{Agent, Config};
use crate::cancel::Cancel;
use anyhow::{Result, ensure};
#[derive(Clone, Copy)]
pub(crate) enum Format<'a> {
    Text,
    JsonSchema(&'a serde_json::Value),
}
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
    runner::run(config, prompt, Format::Text, cancel, &executable, &mut emit)
}

/// Generate a structured result using the agent's native schema support.
pub(crate) fn generate_json(
    config: &Config,
    prompt: &str,
    schema: &serde_json::Value,
    cancel: &Cancel,
) -> Result<serde_json::Value> {
    ensure!(config.enabled, i18n::tr!(AiDisabled));
    config.validate()?;
    let executable = runner::resolve(config.agent)?;
    let text = runner::run(
        config,
        prompt,
        Format::JsonSchema(schema),
        cancel,
        &executable,
        &mut |_| {},
    )?;
    Ok(serde_json::from_str(&text)?)
}

#[cfg(all(test, unix))]
mod tests;
