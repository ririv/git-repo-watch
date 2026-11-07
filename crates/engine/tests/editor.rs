use engine::{
    cancel::Cancel,
    editor::{self, Command, Environment, Interface, Known, Preference},
    state::{Config, Store},
    transport::Git,
};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::Command as Process,
};
use test_support::{Fixture, git, write};
fn environment(path: &Path) -> Environment {
    Environment {
        path: path.as_os_str().to_owned(),
        home: path.to_owned(),
    }
}
fn executable(path: &Path, contents: &str) {
    write(path, contents);
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
}
#[test]
fn editor_preferences_preserve_old_configuration_and_explicit_commands() {
    let home = tempfile::tempdir().unwrap();
    let store = Store::custom(home.path().into());
    let old = "# old profile\ninterval_seconds = 21600\nrepositories = []\nstartup_actions = [\"inspect\"]\nscheduled_actions = [\"inspect\"]\n";
    write(store.config_path(), old);
    let mut config = store.config().unwrap();
    assert_eq!(config.editor, Preference::Git);
    store.save_config(&config).unwrap();
    let saved = fs::read_to_string(store.config_path()).unwrap();
    assert!(saved.starts_with(&old.replace("startup_actions = [\"inspect\"]\n", "")));
    assert!(!saved.contains("editor ="));
    for value in [
        "code",
        "nvim",
        "git",
        "\"/my editors/中文 editor\" --wait",
        "$MY_EDITOR --reuse-window",
    ] {
        config.editor = value.to_owned().try_into().unwrap();
        store.save_config(&config).unwrap();
        assert_eq!(store.config().unwrap().editor.value(), value);
        assert!(
            fs::read_to_string(store.config_path())
                .unwrap()
                .starts_with("# old profile")
        );
    }
    assert_eq!(
        serde_json::to_value(Config::default()).unwrap()["editor"],
        "git"
    );
}
#[test]
fn shell_editor_receives_literal_unicode_space_and_shell_syntax_paths() {
    let f = Fixture::create();
    let directory = f.repo.join("编辑器 folder");
    fs::create_dir(&directory).unwrap();
    let program = directory.join("editor helper");
    let capture = f.repo.join("argv.bin");
    executable(
        &program,
        &format!(
            "#!/bin/sh\nprintf '%s\\0' \"$@\" > '{}'\n",
            capture.display()
        ),
    );
    let editor_command = format!("\"{}\" --flag '中文 参数'", program.display());
    let preference = Preference::Command(Command::parse(editor_command).unwrap());
    for name in [
        "space 中文.ts",
        "literal $(touch BAD); 'quotes'.ts",
        "newline\nfile.ts",
    ] {
        let file = f.repo.join(name);
        write(&file, "content\n");
        let plan = editor::prepare(
            &Git::default(),
            &preference,
            &file,
            &f.repo,
            &environment(&directory),
            &Cancel::default(),
        )
        .unwrap();
        assert_eq!(plan.interface, Interface::Terminal);
        let out = Process::new(plan.program)
            .args(plan.arguments)
            .current_dir(plan.directory)
            .output()
            .unwrap();
        assert!(out.status.success());
        let data = fs::read(&capture).unwrap();
        let expected = format!("--flag\0中文 参数\0{}\0", file.display());
        assert_eq!(data, expected.as_bytes());
        assert!(!f.repo.join("BAD").exists());
        assert_eq!(fs::read_to_string(file).unwrap(), "content\n");
    }
}
#[test]
fn shell_expands_editor_variables_only_when_the_open_is_explicitly_launched() {
    let f = Fixture::create();
    let marker = f.repo.join("launched");
    let program = f.repo.join("helper");
    executable(
        &program,
        &format!("#!/bin/sh\nprintf '%s' \"$1\" > '{}'\n", marker.display()),
    );
    let file = f.repo.join("test.txt");
    write(&file, "test");
    let preference = "$TEST_OPEN_EDITOR".to_owned().try_into().unwrap();
    let plan = editor::prepare(
        &Git::default(),
        &preference,
        &file,
        &f.repo,
        &environment(&f.repo),
        &Cancel::default(),
    )
    .unwrap();
    assert!(!marker.exists());
    let output = Process::new(plan.program)
        .args(plan.arguments)
        .env("TEST_OPEN_EDITOR", &program)
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(fs::read_to_string(marker).unwrap(), file.to_string_lossy());
}
#[test]
fn configured_desktop_editor_launches_without_waiting_for_the_application_lifetime() {
    let f = Fixture::create();
    let code = f.repo.join("code");
    let capture = f.repo.join("opened");
    executable(
        &code,
        &format!(
            "#!/bin/sh\nprintf '%s\\n' \"$1\" > '{}'\n",
            capture.display()
        ),
    );
    let file = f.repo.join("space 中文.ts");
    write(&file, "content");
    let plan = editor::prepare(
        &Git::default(),
        &Preference::Known(Known::Code),
        &file,
        &f.repo,
        &environment(&f.repo),
        &Cancel::default(),
    )
    .unwrap();
    assert_eq!(plan.program, code);
    assert_eq!(plan.interface, Interface::Desktop);
    let mut launch = plan.launch(&Cancel::default()).unwrap();
    let start = std::time::Instant::now();
    loop {
        if launch.poll().unwrap().is_some() {
            break;
        }
        assert!(start.elapsed() < std::time::Duration::from_secs(4));
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert_eq!(
        fs::read_to_string(capture).unwrap().trim_end(),
        file.to_string_lossy()
    );
}
#[test]
fn git_editor_protocol_child() {
    let Some(file) = std::env::var_os("GW_EDITOR_TEST_RESULT") else {
        return;
    };
    let root = PathBuf::from(std::env::var_os("GW_EDITOR_TEST_ROOT").unwrap());
    let value = Git::default().editor(&root, &Cancel::default()).unwrap();
    fs::write(file, value).unwrap();
}
#[test]
fn automatic_editor_uses_git_precedence_and_repository_configuration() {
    let f = Fixture::create();
    let result = f.repo.join("chosen-editor");
    git(&f.repo, &["config", "core.editor", "repo-editor --wait"]);
    for (git_editor, core, visual, editor, expected) in [
        (
            Some("git-editor"),
            true,
            Some("visual-editor"),
            Some("editor"),
            Some("git-editor"),
        ),
        (
            None,
            true,
            Some("visual-editor"),
            Some("editor"),
            Some("repo-editor --wait"),
        ),
        (
            None,
            false,
            Some("visual-editor"),
            Some("editor"),
            Some("visual-editor"),
        ),
        (None, false, None, Some("editor"), Some("editor")),
        (None, false, None, None, None),
    ] {
        if !core {
            let _ = Process::new("/usr/bin/git")
                .args([
                    "-C",
                    f.repo.to_str().unwrap(),
                    "config",
                    "--unset",
                    "core.editor",
                ])
                .output()
                .unwrap();
        }
        let mut command = Process::new(std::env::current_exe().unwrap());
        command
            .args(["--exact", "git_editor_protocol_child", "--nocapture"])
            .env("GW_EDITOR_TEST_ROOT", &f.repo)
            .env("GW_EDITOR_TEST_RESULT", &result)
            .env("TERM", "xterm-256color")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env_remove("GIT_EDITOR")
            .env_remove("VISUAL")
            .env_remove("EDITOR");
        for (key, value) in [
            ("GIT_EDITOR", git_editor),
            ("VISUAL", visual),
            ("EDITOR", editor),
        ] {
            if let Some(value) = value {
                command.env(key, value);
            }
        }
        let output = command.output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let chosen = fs::read_to_string(&result).unwrap();
        if let Some(expected) = expected {
            assert_eq!(chosen, expected);
        } else {
            assert!(!chosen.is_empty());
        }
    }
}

#[test]
fn detects_common_editors_without_launching_them() {
    let f = Fixture::create();
    let environment = Environment::capture(f.temp.path());
    let editors =
        editor::detect(&Git::default(), &f.repo, &environment, &Cancel::default()).unwrap();
    assert!(
        editors
            .iter()
            .any(|entry| entry.preference == Preference::Git)
    );
    for entry in editors {
        println!("{} · {}", entry.label, entry.detail);
    }
}
