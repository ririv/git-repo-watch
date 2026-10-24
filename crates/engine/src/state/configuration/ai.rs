//! Explicit conversion of legacy supplemental instructions into a complete prompt.
use anyhow::{Context, Result, ensure};
use serde::Serialize;
use toml_edit::{DocumentMut, Item};

pub(super) fn migrate(document: &mut DocumentMut) -> Result<()> {
    for name in ["ai", "commit_message"] {
        let Some(table) = document.get_mut(name) else {
            continue;
        };
        let Some(legacy) = table.get("instructions") else {
            continue;
        };
        ensure!(
            table.get("prompt").is_none(),
            i18n::tr!(AgentPromptConflict)
        );
        let instructions = legacy.as_str().context(i18n::tr!(InvalidToml))?;
        let prompt = if instructions.trim().is_empty() {
            crate::ai::Prompt::Default
        } else {
            crate::ai::Prompt::Custom {
                text: format!(
                    "{}\n\n{}",
                    crate::commit_message::DEFAULT_PROMPT.trim(),
                    instructions.trim()
                ),
            }
        };
        prompt.validate()?;
        let mut replacement = encoded(&prompt)?;
        let key = match prompt {
            crate::ai::Prompt::Default => "mode",
            crate::ai::Prompt::Custom { .. } => "text",
        };
        if let Some(old) = legacy.as_value()
            && let Some(new) = replacement[key].as_value_mut()
        {
            *new.decor_mut() = old.decor().clone();
        }
        table
            .as_table_like_mut()
            .context(i18n::tr!(InvalidToml))?
            .remove("instructions");
        table["prompt"] = replacement;
    }
    Ok(())
}
pub(super) fn encoded(prompt: &crate::ai::Prompt) -> Result<Item> {
    #[derive(Serialize)]
    struct Wrapper<'a> {
        prompt: &'a crate::ai::Prompt,
    }
    let table = toml_edit::ser::to_document(&Wrapper { prompt })?
        .remove("prompt")
        .expect("prompt serializes as a table")
        .into_table()
        .expect("prompt serializes as a table");
    Ok(Item::Table(table))
}
pub(super) fn replace(document: &mut DocumentMut, prompt: &crate::ai::Prompt) -> Result<()> {
    if !document.contains_key("ai") {
        document["ai"] = Item::Table(toml_edit::Table::new());
    }
    let mut replacement = encoded(prompt)?;
    if let Some(current) = document["ai"].get("prompt") {
        if let Some(table) = current.as_table() {
            *replacement
                .as_table_mut()
                .expect("prompt table")
                .decor_mut() = table.decor().clone();
        }
        for key in ["mode", "text"] {
            if let Some(old) = current.get(key).and_then(Item::as_value)
                && let Some(new) = replacement.get_mut(key).and_then(Item::as_value_mut)
            {
                *new.decor_mut() = old.decor().clone();
            }
        }
    }
    document["ai"]["prompt"] = replacement;
    Ok(())
}
