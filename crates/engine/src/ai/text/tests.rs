use super::*;
use serde_json::{Value, json};
use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf};

fn agent(root: &std::path::Path, claude_result: Value) -> PathBuf {
    fs::write(root.join("claude.json"), format!("{claude_result}\n")).unwrap();
    let item = json!({"type":"agentMessage","id":"message","text":"{\"commits\":[{\"message\":\"fix: 示例\",\"units\":[\"W1\"]}]}","phase":"final_answer"});
    let events = [
        json!({"id":3,"result":{"turn":{"id":"turn"}}}),
        json!({"method":"item/started","params":{"threadId":"thread","turnId":"turn","item":item}}),
        json!({"method":"item/completed","params":{"threadId":"thread","turnId":"turn","item":item}}),
        json!({"method":"turn/completed","params":{"threadId":"thread","turn":{"id":"turn","status":"completed"}}}),
    ];
    fs::write(
        root.join("codex.json"),
        events
            .iter()
            .map(|event| format!("{event}\n"))
            .collect::<String>(),
    )
    .unwrap();
    let path = root.join("mock agent");
    fs::write(
        &path,
        r#"#!/bin/sh
set -eu
base=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
printf '%s\n' "$@" > "$base/arguments"
if [ "$1" = 'app-server' ]; then
  IFS= read -r initialize
  printf '%s\n' '{"id":1,"result":{}}'
  IFS= read -r initialized
  IFS= read -r thread
  printf '%s\n' '{"id":2,"result":{"thread":{"id":"thread"}}}'
  IFS= read -r turn
  printf '%s\n' "$turn" > "$base/turn"
  cat "$base/codex.json"
else
  cat > "$base/prompt"
  cat "$base/claude.json"
fi
"#,
    )
    .unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
    path
}
#[test]
fn adapters_send_the_same_schema_and_read_the_native_result() {
    let schema = json!({"type":"object","required":["commits"],"additionalProperties":false,"properties":{"commits":{"type":"array","items":{"type":"object","required":["message","units"],"additionalProperties":false,"properties":{"message":{"type":"string"},"units":{"type":"array","items":{"type":"string","enum":["W1"]}}}}}}});
    let expected = json!({"commits":[{"message":"fix: 示例","units":["W1"]}]});
    for agent_kind in [Agent::Codex, Agent::Claude] {
        let root = tempfile::tempdir().unwrap();
        let program = agent(
            root.path(),
            json!({"type":"result","subtype":"success","is_error":false,"result":"ordinary text must not be parsed as the plan","structured_output":expected}),
        );
        let config = Config {
            agent: agent_kind,
            ..Config::default()
        };
        let text = runner::run(
            &config,
            "plan changes",
            Format::JsonSchema(&schema),
            &Cancel::default(),
            &program,
            &mut |_| {},
        )
        .unwrap();
        assert_eq!(serde_json::from_str::<Value>(&text).unwrap(), expected);
        match agent_kind {
            Agent::Codex => {
                let turn: Value =
                    serde_json::from_str(&fs::read_to_string(root.path().join("turn")).unwrap())
                        .unwrap();
                assert_eq!(turn["params"]["outputSchema"], schema);
                assert_eq!(turn["params"]["input"][0]["text"], "plan changes");
            }
            Agent::Claude => {
                let arguments = fs::read_to_string(root.path().join("arguments")).unwrap();
                let args = arguments.lines().collect::<Vec<_>>();
                let index = args.iter().position(|arg| *arg == "--json-schema").unwrap();
                assert_eq!(
                    serde_json::from_str::<Value>(args[index + 1]).unwrap(),
                    schema
                );
            }
        }
    }
}
#[test]
fn claude_missing_structured_result_is_rejected_even_when_text_contains_valid_json() {
    let root = tempfile::tempdir().unwrap();
    let program = agent(
        root.path(),
        json!({"type":"result","subtype":"success","is_error":false,"result":"{\"commits\":[]}"}),
    );
    let config = Config {
        agent: Agent::Claude,
        ..Config::default()
    };
    assert!(
        runner::run(
            &config,
            "plan",
            Format::JsonSchema(&json!({"type":"object"})),
            &Cancel::default(),
            &program,
            &mut |_| {}
        )
        .is_err()
    );
}
#[test]
fn codex_schema_rejection_is_reported_without_retrying_as_text() {
    let root = tempfile::tempdir().unwrap();
    let program = agent(root.path(), json!({}));
    fs::write(
        root.path().join("codex.json"),
        "{\"id\":3,\"error\":{\"code\":-32602,\"message\":\"outputSchema unsupported\"}}\n",
    )
    .unwrap();
    let error = runner::run(
        &Config::default(),
        "plan",
        Format::JsonSchema(&json!({"type":"object"})),
        &Cancel::default(),
        &program,
        &mut |_| {},
    )
    .unwrap_err();
    assert!(format!("{error:#}").contains("outputSchema unsupported"));
}
