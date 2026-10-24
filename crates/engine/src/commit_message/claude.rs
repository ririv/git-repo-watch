//! Claude JSON events expose real text deltas and an authoritative final result.
use super::{Config, Update, wire};
use crate::{cancel::Cancel, process::conversation};
use anyhow::{Result, ensure};
use std::{io::Write, path::Path, process::Command, time::Duration};

pub(super) fn run(
    config: &Config,
    prompt: &str,
    cancel: &Cancel,
    executable: &Path,
    directory: &Path,
    emit: &mut (impl FnMut(Update) + Send),
) -> Result<String> {
    let mut command = Command::new(executable);
    command.current_dir(directory).args([
        "--print",
        "--output-format",
        "stream-json",
        "--verbose",
        "--include-partial-messages",
        "--tools",
        "",
        "--strict-mcp-config",
        "--mcp-config",
        "{\"mcpServers\":{}}",
        "--no-session-persistence",
        "--disable-slash-commands",
        "--no-chrome",
        "--setting-sources",
        "user",
        "--settings",
        "{\"disableAllHooks\":true}",
    ]);
    if !config.program().model.is_empty() {
        command.args(["--model", &config.program().model]);
    }
    conversation::run(
        command,
        cancel,
        Duration::from_secs(120),
        conversation::Completion::Process,
        |input, mut output| {
            output.write_all(prompt.as_bytes())?;
            drop(output);
            let mut reader = wire::Reader::new(input);
            let mut bytes = 0;
            loop {
                let event = reader.next()?;
                match event["type"].as_str() {
                    Some("stream_event") if event["event"]["delta"]["type"] == "text_delta" => {
                        let delta = wire::text(&event["event"]["delta"]["text"])?;
                        bytes += delta.len();
                        ensure!(
                            bytes <= super::MESSAGE_LIMIT,
                            i18n::tr!(AgentOutputTooLarge)
                        );
                        emit(Update::Append(delta.into()));
                    }
                    Some("result") => {
                        ensure!(
                            event["subtype"] == "success" && event["is_error"] == false,
                            "Claude Code: {event}"
                        );
                        let message = wire::text(&event["result"])?;
                        ensure!(
                            message.len() <= super::MESSAGE_LIMIT,
                            i18n::tr!(AgentOutputTooLarge)
                        );
                        emit(Update::Replace(message.into()));
                        return Ok(message.into());
                    }
                    _ => {}
                }
            }
        },
    )
}
