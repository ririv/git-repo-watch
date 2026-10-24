//! One ephemeral App Server turn; only agent-message events reach the draft.
use super::{Config, Update, wire};
use crate::{cancel::Cancel, process::conversation};
use anyhow::{Context, Result, bail, ensure};
use serde_json::{Value, json};
use std::{path::Path, process::Command, time::Duration};

pub(super) fn run(
    config: &Config,
    prompt: &str,
    cancel: &Cancel,
    executable: &Path,
    directory: &Path,
    emit: &mut (impl FnMut(Update) + Send),
) -> Result<String> {
    let mut command = Command::new(executable);
    command
        .arg("app-server")
        .arg("--stdio")
        .current_dir(directory);
    for option in [
        "approval_policy=\"never\"",
        "sandbox_mode=\"read-only\"",
        "features.shell_tool=false",
        "features.apply_patch_freeform=false",
        "features.multi_agent=false",
        "features.apps=false",
        "features.plugins=false",
        "mcp_servers={}",
        "web_search=\"disabled\"",
    ] {
        command.args(["-c", option]);
    }
    conversation::run(
        command,
        cancel,
        Duration::from_secs(120),
        conversation::Completion::Protocol,
        |input, mut output| {
            wire::send(
                &mut output,
                json!({"id":1,"method":"initialize","params":{"clientInfo":{"name":identity::COMMAND,"title":identity::DISPLAY_NAME,"version":env!("CARGO_PKG_VERSION")}}}),
            )?;
            let mut reader = wire::Reader::new(input);
            let mut stage = Stage::Initialize;
            let mut message = Message::Empty;
            loop {
                let event = reader.next()?;
                if let Some(error) = event.get("error") {
                    bail!("Codex: {error}");
                }
                if let Some(id) = event.get("id") {
                    ensure!(
                        event.get("method").is_none(),
                        "Codex: {}",
                        i18n::tr!(AgentStreamUnexpectedRequest)
                    );
                    match (&stage, id.as_u64()) {
                        (Stage::Initialize, Some(1)) => {
                            wire::send(&mut output, json!({"method":"initialized","params":{}}))?;
                            let mut params = json!({"cwd":directory,"ephemeral":true,"approvalPolicy":"never","sandbox":"read-only"});
                            if !config.program().model.is_empty() {
                                params["model"] = config.program().model.clone().into();
                            }
                            wire::send(
                                &mut output,
                                json!({"id":2,"method":"thread/start","params":params}),
                            )?;
                            stage = Stage::Thread;
                        }
                        (Stage::Thread, Some(2)) => {
                            let thread = wire::text(&event["result"]["thread"]["id"])?.to_owned();
                            wire::send(
                                &mut output,
                                json!({"id":3,"method":"turn/start","params":{"threadId":thread,"input":[{"type":"text","text":prompt}],"approvalPolicy":"never"}}),
                            )?;
                            stage = Stage::Turn { thread };
                        }
                        (Stage::Turn { thread }, Some(3)) => {
                            stage = Stage::Active {
                                thread: thread.clone(),
                                turn: wire::text(&event["result"]["turn"]["id"])?.into(),
                            }
                        }
                        (Stage::Active { turn, .. }, Some(3)) => ensure!(
                            event["result"]["turn"]["id"] == *turn,
                            i18n::tr!(AgentMessageInvalid)
                        ),
                        _ => bail!("{}: {event}", i18n::tr!(AgentMessageInvalid)),
                    }
                    continue;
                }
                let params = &event["params"];
                let method = event["method"]
                    .as_str()
                    .context(i18n::tr!(AgentMessageInvalid))?;
                if method == "turn/started"
                    && let Stage::Turn { thread } = &stage
                {
                    ensure!(
                        params["threadId"] == *thread,
                        i18n::tr!(AgentMessageInvalid)
                    );
                    stage = Stage::Active {
                        thread: thread.clone(),
                        turn: wire::text(&params["turn"]["id"])?.into(),
                    };
                    continue;
                }
                let Stage::Active { thread, turn } = &stage else {
                    continue;
                };
                if params.get("threadId").is_some() {
                    ensure!(
                        params["threadId"] == *thread,
                        i18n::tr!(AgentMessageInvalid)
                    );
                }
                if params.get("turnId").is_some() {
                    ensure!(params["turnId"] == *turn, i18n::tr!(AgentMessageInvalid));
                }
                match method {
                    "item/started" if agent_message(&params["item"]) => {
                        message = Message::Streaming {
                            id: wire::text(&params["item"]["id"])?.into(),
                            text: String::new(),
                        };
                        emit(Update::Replace(String::new()));
                    }
                    "item/agentMessage/delta" => {
                        if let Message::Streaming { id, text } = &mut message {
                            if params["itemId"] != *id {
                                continue;
                            }
                            let delta = wire::text(&params["delta"])?;
                            ensure!(
                                text.len() + delta.len() <= super::MESSAGE_LIMIT,
                                i18n::tr!(AgentOutputTooLarge)
                            );
                            text.push_str(delta);
                            emit(Update::Append(delta.into()));
                        }
                    }
                    "item/completed" if agent_message(&params["item"]) => {
                        let text = wire::text(&params["item"]["text"])?;
                        ensure!(
                            text.len() <= super::MESSAGE_LIMIT,
                            i18n::tr!(AgentOutputTooLarge)
                        );
                        message = Message::Complete(text.into());
                        emit(Update::Replace(text.into()));
                    }
                    "turn/completed" => {
                        ensure!(
                            params["turn"]["id"] == *turn
                                && params["turn"]["status"] == "completed",
                            "Codex: {}",
                            params["turn"]
                        );
                        return match message {
                            Message::Complete(text) => Ok(text),
                            _ => bail!(i18n::tr!(AgentStreamIncomplete)),
                        };
                    }
                    "error" if params["willRetry"] != true => bail!("Codex: {}", params["error"]),
                    _ => {}
                }
            }
        },
    )
}
fn agent_message(item: &Value) -> bool {
    item["type"] == "agentMessage"
        && item["phase"] != "commentary"
        && item["phase"] != "partial_answer"
}
enum Stage {
    Initialize,
    Thread,
    Turn { thread: String },
    Active { thread: String, turn: String },
}
enum Message {
    Empty,
    Streaming { id: String, text: String },
    Complete(String),
}
