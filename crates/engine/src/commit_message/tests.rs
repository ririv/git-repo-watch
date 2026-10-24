use super::*;
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
        runner::run(config, prompt, cancel, program)
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
        let program = executable(
            directory.path(),
            r#"
printf '%s\n' "$@" > "$base/arguments"
pwd > "$base/cwd"
cat > "$base/prompt"
output=''
while [ "$#" -gt 0 ]; do
  if [ "$1" = '--output-last-message' ]; then shift; output=$1; fi
  shift
done
if [ -n "$output" ]; then printf '%s\n' 'feat: add staged content' > "$output";
else printf '%s\n' '{"subtype":"success","is_error":false,"result":"feat: add staged content"}'; fi
"#,
        );
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
        assert!(!prompt.contains("Write a Git commit subject"));
        assert!(!prompt.contains("unstaged-secret") && !prompt.contains("untracked-secret"));
        assert!(!prompt.contains(f.repo.to_str().unwrap()));
        let arguments = fs::read_to_string(directory.path().join("arguments")).unwrap();
        assert!(arguments.contains("literal $(echo should-not-run) model\n"));
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
    let program = executable(
        directory.path(),
        r#"
cat > /dev/null
: > "$base/ready"
while [ ! -f "$base/release" ]; do sleep 0.01; done
while [ "$1" != '--output-last-message' ]; do shift; done
printf 'feat: old snapshot\n' > "$2"
"#,
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
        "cat >/dev/null; printf 'login required' >&2; exit 7",
        "cat >/dev/null; while [ \"$1\" != '--output-last-message' ]; do shift; done; printf 'first\\nsecond\\n' > \"$2\"",
        "cat >/dev/null; while [ \"$1\" != '--output-last-message' ]; do shift; done; : > \"$2\"",
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
    let directory = tempfile::tempdir().unwrap();
    let program = executable(directory.path(), "cat >/dev/null; sleep 30");
    let cancel = Cancel::default();
    let task_cancel = cancel.clone();
    let cwd = directory.path().to_owned();
    let executable_path = program.clone();
    let started = Instant::now();
    let handle = thread::spawn(move || {
        crate::process::run_with_input(
            &executable_path,
            &[],
            &cwd,
            b"input",
            &task_cancel,
            Duration::from_secs(30),
            4096,
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
        crate::process::run_with_input(
            &program,
            &[],
            directory.path(),
            b"input",
            &Cancel::default(),
            Duration::from_millis(40),
            4096
        )
        .is_err()
    );
    let program = executable(directory.path(), "cat >/dev/null; yes overflow");
    assert_eq!(
        crate::process::run_with_input(
            &program,
            &[],
            directory.path(),
            b"input",
            &Cancel::default(),
            Duration::from_secs(5),
            4096
        )
        .unwrap_err()
        .to_string(),
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
        generate(&plan, &config, &Cancel::default())
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
