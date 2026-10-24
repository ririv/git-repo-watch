//! Generate editable commit subjects through explicitly selected local agents.
mod context;
mod runner;
use crate::ai::{Agent, Config};
use crate::{
    cancel::Cancel,
    local::{self, review::CommitPlan},
};
use anyhow::{Result, ensure};
pub const DEFAULT_PROMPT: &str = include_str!("default_prompt.txt");

pub fn generate(plan: &CommitPlan, config: &Config, cancel: &Cancel) -> Result<String> {
    ensure!(config.enabled, i18n::tr!(AiDisabled));
    let executable = runner::resolve(config.agent)?;
    generate_using(plan, config, cancel, |config, prompt, cancel| {
        runner::run(config, prompt, cancel, &executable)
    })
}
fn generate_using(
    plan: &CommitPlan,
    config: &Config,
    cancel: &Cancel,
    run: impl FnOnce(&Config, &str, &Cancel) -> Result<String>,
) -> Result<String> {
    ensure!(config.enabled, i18n::tr!(AiDisabled));
    config.validate()?;
    let context = context::read(plan, cancel)?;
    let language = match i18n::language() {
        i18n::Language::English => "English",
        i18n::Language::Chinese => "Simplified Chinese",
    };
    let prompt = format!(
        "{}\n\n<context>\nInterface language: {language}\n\n{context}\n</context>",
        config.prompt.text(DEFAULT_PROMPT).trim()
    );
    let message = run(config, &prompt, cancel)?;
    cancel.check()?;
    local::review::validate(plan)?;
    let message = message.trim();
    ensure!(
        !message.is_empty()
            && message.chars().count() <= 500
            && !message.chars().any(char::is_control)
            && !message.starts_with("```"),
        i18n::tr!(AgentMessageInvalid)
    );
    Ok(message.to_owned())
}

#[cfg(test)]
mod tests;
