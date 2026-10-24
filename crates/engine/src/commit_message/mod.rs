//! Generate editable commit messages through explicitly selected local agents.
mod context;
pub use crate::ai::text::Update;

use crate::ai::Config;
pub use crate::ai::text::MESSAGE_LIMIT;
use crate::{
    cancel::Cancel,
    local::{self, review::CommitPlan},
};
use anyhow::{Result, ensure};
pub const DEFAULT_PROMPT: &str = include_str!("default_prompt.txt");

pub fn generate(
    plan: &CommitPlan,
    config: &Config,
    cancel: &Cancel,
    mut emit: impl FnMut(Update) + Send,
) -> Result<String> {
    ensure!(config.enabled, i18n::tr!(AiDisabled));
    generate_using(plan, config, cancel, |config, prompt, cancel| {
        crate::ai::text::generate(config, prompt, cancel, &mut emit)
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
    let message = message.replace("\r\n", "\n").replace('\r', "\n");
    let message = message.trim();
    ensure!(
        !message.is_empty()
            && message.len() <= MESSAGE_LIMIT
            && !message
                .chars()
                .any(|c| c.is_control() && !matches!(c, '\n' | '\t'))
            && !message.starts_with("```"),
        i18n::tr!(AgentMessageInvalid)
    );
    Ok(message.to_owned())
}

#[cfg(test)]
mod tests;
