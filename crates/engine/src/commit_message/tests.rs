use super::*;
use crate::ai::{
    Agent,
    text::{runner, wire},
};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    thread,
    time::{Duration, Instant},
};
use test_support as support;

fn generate_program(
    plan: &CommitPlan,
    config: &Config,
    program: &Path,
    cancel: &Cancel,
) -> anyhow::Result<String> {
    super::generate_using(plan, config, cancel, |config, prompt, cancel| {
        runner::run(config, prompt, cancel, program, &mut |_| {})
    })
}
fn executable(directory: &Path, script: &str) -> PathBuf {
    let path = directory.join("mock agent 中文");
    fs::write(
        &path,
        format!(
            "#!/bin/sh\nset -eu\nbase=$(CDPATH= cd -- \"$(dirname -- \"$0\")\" && pwd)\n{script}"
        ),
    )
    .unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
    path
}
fn mock_agent(directory: &Path, message: &str, pause: &str) -> PathBuf {
    use serde_json::json;
    let split = message
        .char_indices()
        .nth(12)
        .map_or(message.len(), |(index, _)| index);
    let (first, rest) = message.split_at(split);
    let item = json!({"type":"agentMessage","id":"message","text":message,"phase":"final_answer"});
    let codex_first = [
        json!({"id":3,"result":{"turn":{"id":"turn"}}}),
        json!({"method":"item/started","params":{"threadId":"thread","turnId":"turn","item":item}}),
        json!({"method":"item/agentMessage/delta","params":{"threadId":"thread","turnId":"turn","itemId":"message","delta":first}}),
    ];
    let codex_rest = [
        json!({"method":"item/agentMessage/delta","params":{"threadId":"thread","turnId":"turn","itemId":"message","delta":rest}}),
        json!({"method":"item/completed","params":{"threadId":"thread","turnId":"turn","item":item}}),
        json!({"method":"turn/completed","params":{"threadId":"thread","turn":{"id":"turn","status":"completed","items":[]}}}),
    ];
    let claude_first =
        [json!({"type":"stream_event","event":{"delta":{"type":"text_delta","text":first}}})];
    let claude_rest = [
        json!({"type":"stream_event","event":{"delta":{"type":"text_delta","text":rest}}}),
        json!({"type":"result","subtype":"success","is_error":false,"result":message}),
    ];
    for (name, events) in [
        ("codex-first", codex_first.as_slice()),
        ("codex-rest", codex_rest.as_slice()),
        ("claude-first", claude_first.as_slice()),
        ("claude-rest", claude_rest.as_slice()),
    ] {
        fs::write(
            directory.join(name),
            events.iter().map(|e| format!("{e}\n")).collect::<String>(),
        )
        .unwrap();
    }
    executable(
        directory,
        &format!(
            r#"
printf '%s\n' "$@" > "$base/arguments"
pwd > "$base/cwd"
if [ "$1" = 'app-server' ]; then
  IFS= read -r initialize
  printf '%s\n' '{{"id":1,"result":{{}}}}'
  IFS= read -r initialized
  IFS= read -r thread
  printf '%s\n' "$thread" > "$base/thread"
  printf '%s\n' '{{"id":2,"result":{{"thread":{{"id":"thread"}}}}}}'
  IFS= read -r turn
  printf '%s\n' "$turn" > "$base/prompt"
  cat "$base/codex-first"
  {pause}
  cat "$base/codex-rest"
else
  cat > "$base/prompt"
  cat "$base/claude-first"
  {pause}
  cat "$base/claude-rest"
fi
"#
        ),
    )
}

fn staged(f: &support::Fixture) -> CommitPlan {
    support::write(f.repo.join("中文 staged.txt"), "staged-only-value\n");
    support::git(&f.repo, &["add", "--", "中文 staged.txt"]);
    local::review::prepare(&f.repo).unwrap()
}
fn await_file(path: &Path) {
    let start = Instant::now();
    while !path.exists() {
        assert!(start.elapsed() < Duration::from_secs(8));
        thread::sleep(Duration::from_millis(5));
    }
}
#[test]
fn both_adapters_send_only_staged_context_and_keep_repository_untouched() {
    let f = support::Fixture::create();
    let plan = staged(&f);
    support::write(f.repo.join("中文 staged.txt"), "unstaged-secret\n");
    support::write(f.repo.join("untracked.txt"), "untracked-secret\n");
    let index = fs::read(f.repo.join(".git/index")).unwrap();
    let head = support::git(&f.repo, &["rev-parse", "HEAD"]);
    for agent in [Agent::Codex, Agent::Claude] {
        let directory = tempfile::tempdir().unwrap();
        let program = mock_agent(directory.path(), "feat: add staged content", "");
        let mut config = Config {
            agent,
            prompt: crate::ai::Prompt::Custom {
                text: "Use conventional commits".into(),
            },
            ..Config::default()
        };
        config.program_mut().model = "literal $(echo should-not-run) model".into();
        assert_eq!(
            generate_program(&plan, &config, &program, &Cancel::default()).unwrap(),
            "feat: add staged content"
        );
        let prompt = fs::read_to_string(directory.path().join("prompt")).unwrap();
        assert!(
            prompt.contains("staged-only-value") && prompt.contains("Use conventional commits")
        );
        assert!(!prompt.contains(DEFAULT_PROMPT.trim()));
        assert!(!prompt.contains("unstaged-secret") && !prompt.contains("untracked-secret"));
        assert!(!prompt.contains(f.repo.to_str().unwrap()));
        let arguments = fs::read_to_string(directory.path().join("arguments")).unwrap();
        if agent == Agent::Codex {
            assert!(
                fs::read_to_string(directory.path().join("thread"))
                    .unwrap()
                    .contains("literal $(echo should-not-run) model")
            );
        } else {
            assert!(arguments.contains("literal $(echo should-not-run) model\n"));
        }
        assert!(!arguments.contains("staged-only-value"));
        assert_ne!(
            fs::read_to_string(directory.path().join("cwd"))
                .unwrap()
                .trim(),
            f.repo.to_str().unwrap()
        );
    }
    assert_eq!(fs::read(f.repo.join(".git/index")).unwrap(), index);
    assert_eq!(support::git(&f.repo, &["rev-parse", "HEAD"]), head);
    assert_eq!(
        fs::read_to_string(f.repo.join("中文 staged.txt")).unwrap(),
        "unstaged-secret\n"
    );
}
#[test]
fn context_handles_first_commit_and_binary_metadata() {
    let directory = tempfile::tempdir().unwrap();
    support::init(directory.path());
    fs::write(directory.path().join("binary.dat"), [0, 1, 2, 3]).unwrap();
    support::git(directory.path(), &["add", "."]);
    let plan = local::review::prepare(directory.path()).unwrap();
    let text = context::read(&plan, &Cancel::default()).unwrap();
    assert!(text.contains("binary.dat") && text.contains("Binary files"));
}
#[test]
fn stale_staging_is_rejected_after_agent_completes() {
    let f = support::Fixture::create();
    let plan = staged(&f);
    let directory = tempfile::tempdir().unwrap();
    let program = mock_agent(
        directory.path(),
        "feat: old snapshot",
        r#": > "$base/ready"; while [ ! -f "$base/release" ]; do sleep 0.01; done"#,
    );
    let config = Config::default();
    let handle =
        thread::spawn(move || generate_program(&plan, &config, &program, &Cancel::default()));
    await_file(&directory.path().join("ready"));
    support::write(f.repo.join("new.txt"), "new staging\n");
    support::git(&f.repo, &["add", "new.txt"]);
    fs::write(directory.path().join("release"), "").unwrap();
    let error = handle.join().unwrap().unwrap_err().to_string();
    assert_eq!(error, i18n::tr!(StagedContentsChangedReviewTheCommitAgain));
}
#[test]
fn generation_reports_cli_failures_and_invalid_results_without_guessing() {
    let f = support::Fixture::create();
    let plan = staged(&f);
    for script in [
        "printf 'login required' >&2; exit 7",
        "printf 'not-json\\n'; exit 0",
        "exit 0",
    ] {
        let directory = tempfile::tempdir().unwrap();
        let config = Config::default();
        let program = executable(directory.path(), script);
        assert!(generate_program(&plan, &config, &program, &Cancel::default()).is_err());
    }
    let config = Config::default();
    let program = Path::new("/definitely-missing-agent");
    assert!(generate_program(&plan, &config, program, &Cancel::default()).is_err());
}
#[test]
fn cancellation_timeout_and_output_limits_terminate_background_processes() {
    use crate::process::conversation::{self, Completion};
    let directory = tempfile::tempdir().unwrap();
    let program = executable(directory.path(), "cat >/dev/null; sleep 30");
    let cancel = Cancel::default();
    let task_cancel = cancel.clone();
    let executable_path = program.clone();
    let started = Instant::now();
    let handle = thread::spawn(move || {
        conversation::run(
            std::process::Command::new(executable_path),
            &task_cancel,
            Duration::from_secs(30),
            Completion::Protocol,
            |input, output| {
                drop(output);
                wire::Reader::new(input).next()
            },
        )
    });
    thread::sleep(Duration::from_millis(60));
    cancel.cancel();
    assert!(
        handle
            .join()
            .unwrap()
            .unwrap_err()
            .downcast_ref::<crate::cancel::Cancelled>()
            .is_some()
    );
    assert!(started.elapsed() < Duration::from_secs(3));
    assert!(
        conversation::run(
            std::process::Command::new(&program),
            &Cancel::default(),
            Duration::from_millis(40),
            Completion::Protocol,
            |input, output| {
                drop(output);
                wire::Reader::new(input).next()
            }
        )
        .is_err()
    );
    let program = executable(
        directory.path(),
        r#"printf '\"'; head -c 1100000 /dev/zero | tr '\000' 'a'; printf '\"\n'"#,
    );
    let error = conversation::run(
        std::process::Command::new(&program),
        &Cancel::default(),
        Duration::from_secs(5),
        Completion::Protocol,
        |input, output| {
            drop(output);
            wire::Reader::new(input).next()
        },
    )
    .unwrap_err();
    assert_eq!(
        error.root_cause().to_string(),
        i18n::tr!(AgentOutputTooLarge)
    );
}
#[test]
fn oversized_staged_patch_requires_explicitly_smaller_scope() {
    let f = support::Fixture::create();
    support::write(
        f.repo.join("large.txt"),
        &"line with content\n".repeat(20000),
    );
    support::git(&f.repo, &["add", "large.txt"]);
    let plan = local::review::prepare(&f.repo).unwrap();
    assert_eq!(
        context::read(&plan, &Cancel::default())
            .unwrap_err()
            .to_string(),
        i18n::tr!(AgentDiffTooLarge)
    );
}
#[test]
fn disabled_ai_rejects_generation_before_invoking_an_agent() {
    let f = support::Fixture::create();
    let plan = staged(&f);
    let config = Config {
        enabled: false,
        ..Default::default()
    };
    assert_eq!(
        generate(&plan, &config, &Cancel::default(), |_| {})
            .unwrap_err()
            .to_string(),
        i18n::tr!(AiDisabled)
    );
    assert!(
        super::generate_using(&plan, &config, &Cancel::default(), |_, _, _| panic!(
            "disabled AI must not launch"
        ))
        .is_err()
    );
}

#[test]
fn selected_prompt_is_used_whole_and_restoring_default_restores_builtin_rules() {
    let f = support::Fixture::create();
    let plan = staged(&f);
    for (prompt, expected) in [
        (crate::ai::Prompt::Default, DEFAULT_PROMPT.trim()),
        (
            crate::ai::Prompt::Custom {
                text: "只输出简短标题。\n保留格式。".into(),
            },
            "只输出简短标题。\n保留格式。",
        ),
    ] {
        let config = Config {
            prompt,
            ..Default::default()
        };
        let message = super::generate_using(&plan, &config, &Cancel::default(), |_, input, _| {
            let (actual, context) = input.split_once("\n\n<context>\n").unwrap();
            assert_eq!(actual, expected);
            assert!(
                context.contains("Interface language:") && context.contains("staged-only-value")
            );
            Ok("feat: staged content".into())
        })
        .unwrap();
        assert_eq!(message, "feat: staged content");
    }
}

#[test]
fn both_adapters_preserve_multiline_messages_beyond_the_old_subject_limit() {
    for agent in [Agent::Codex, Agent::Claude] {
        let f = support::Fixture::create();
        let plan = staged(&f);
        let directory = tempfile::tempdir().unwrap();
        let body = "说明行为变化与设计原因，不截断生成正文。".repeat(40);
        let message = format!("feat: 完整提交消息\n\n{body}\n\n- 保留分段\n- 保留中文");
        let program = mock_agent(directory.path(), &message.replace('\n', "\r\n"), "");
        let config = Config {
            agent,
            ..Default::default()
        };
        let actual = generate_program(&plan, &config, &program, &Cancel::default()).unwrap();
        assert_eq!(actual, message);
        crate::operations::commit::execute(
            &crate::transport::Git::default(),
            &plan,
            &actual,
            &Cancel::default(),
        )
        .unwrap();
        assert_eq!(
            support::git(&f.repo, &["log", "-1", "--format=%B"]),
            message
        );
    }
}

#[test]
fn multiline_generation_rejects_empty_control_fenced_and_oversized_output_without_truncation() {
    let f = support::Fixture::create();
    let plan = staged(&f);
    for output in [
        " \n\t".into(),
        "title\n\ninvalid\0body".into(),
        "```\nmessage\n```".into(),
        "a".repeat(MESSAGE_LIMIT + 1),
    ] {
        assert!(
            generate_using(&plan, &Config::default(), &Cancel::default(), |_, _, _| Ok(
                output
            ))
            .is_err()
        );
    }
    let message = "a".repeat(MESSAGE_LIMIT);
    assert_eq!(
        generate_using(&plan, &Config::default(), &Cancel::default(), |_, _, _| Ok(
            message.clone()
        ))
        .unwrap(),
        message
    );
}

#[test]
fn real_deltas_arrive_before_completion_and_cancel_reaps_the_stream() {
    for agent in [Agent::Codex, Agent::Claude] {
        let f = support::Fixture::create();
        let plan = staged(&f);
        let directory = tempfile::tempdir().unwrap();
        let program = mock_agent(
            directory.path(),
            "feat: actual stream\n\n正文分段",
            r#": > "$base/ready"; while [ ! -f "$base/release" ]; do sleep 0.01; done"#,
        );
        let config = Config {
            agent,
            ..Default::default()
        };
        let cancel = Cancel::default();
        let task_cancel = cancel.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        let handle = thread::spawn(move || {
            generate_using(&plan, &config, &task_cancel, |config, prompt, cancel| {
                runner::run(config, prompt, cancel, &program, &mut |update| {
                    tx.send(update).unwrap();
                })
            })
        });
        await_file(&directory.path().join("ready"));
        loop {
            if matches!(rx.recv_timeout(Duration::from_secs(3)).unwrap(), Update::Append(text) if !text.is_empty())
            {
                break;
            }
        }
        assert!(!handle.is_finished());
        cancel.cancel();
        assert!(
            handle
                .join()
                .unwrap()
                .unwrap_err()
                .downcast_ref::<crate::cancel::Cancelled>()
                .is_some()
        );
    }
}
