use super::*;
use crate::clipboard::{Scope, Writer};

fn capture(s: &mut Screen) -> mpsc::Receiver<String> {
    let (tx, rx) = mpsc::channel();
    s.clipboard = Writer::with_sink(move |text| {
        tx.send(text)?;
        Ok(())
    });
    rx
}
fn copied(s: &mut Screen, rx: &mpsc::Receiver<String>) -> String {
    let text = rx.recv_timeout(Duration::from_secs(8)).unwrap();
    let started = Instant::now();
    while !s.poll_clipboard() {
        assert!(started.elapsed() < Duration::from_secs(8));
        thread::sleep(Duration::from_millis(2));
    }
    assert!(
        s.message.contains(i18n::tr!(CopySucceeded)),
        "{}",
        s.message
    );
    text
}
fn local_copy(s: &mut Screen) {
    key(s, KeyCode::Char('/'));
    s.paste("copy");
    key(s, KeyCode::Enter);
}

fn comparable_patch(patch: &str) -> String {
    // Git CLI 在带空格的文件头后补 Tab；libgit2 不补，补丁语义相同。
    patch
        .lines()
        .map(|line| {
            if line.starts_with("--- ") || line.starts_with("+++ ") {
                line.trim_end_matches('\t')
            } else {
                line
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}
fn assert_patch_applies(repo: &std::path::Path, patch: &str) {
    use std::io::Write;
    let mut process = std::process::Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["apply", "--check", "--cached", "-"])
        .stdin(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    process
        .stdin
        .take()
        .unwrap()
        .write_all(patch.as_bytes())
        .unwrap();
    let output = process.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn repository_copy_freezes_the_selected_path_and_global_search_hides_other_views() {
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let path = format!("/example/{} 中文 项目", "long-name-".repeat(25));
        let (_home, mut s) = fixture(vec![path.clone().into(), "/other/repo".into()]);
        let config = std::fs::read(s.service.store.config_path()).unwrap();
        s.focus(Some(path.clone().into()));
        let rx = capture(&mut s);
        s.paste("//copy");
        assert_eq!(
            s.search_commands()
                .unwrap()
                .iter()
                .filter(|c| matches!(c.action, Action::Copy(_)))
                .map(|c| c.name)
                .collect::<Vec<_>>(),
            ["repo copy"]
        );
        s.focus(Some("/other/repo".into()));
        for (w, h) in [(60, 18), (100, 28), (180, 36)] {
            let text = render(&mut s, w, h);
            assert!(
                text.contains("/repo copy") && text.contains(i18n::tr!(CopyRepositoryPath)),
                "{text}"
            );
        }
        key(&mut s, KeyCode::Enter);
        assert_eq!(copied(&mut s, &rx), path);
        assert_eq!(s.selected_path(), Some("/other/repo".into()));
        assert_eq!(
            std::fs::read(s.service.store.config_path()).unwrap(),
            config
        );
        assert!(s.service.store.report().unwrap().is_none());
        assert!(s.repository_tasks.is_empty() && s.check_task.is_none());
    }
}

#[test]
fn changes_copy_uses_file_marks_or_the_focused_diff_and_retains_immutable_drafts() {
    let f = support::Fixture::create();
    support::write(f.repo.join("中文 file.ts"), "export const before = 1;\n");
    support::write(f.repo.join("second.rs"), "fn before() {}\n");
    support::git(&f.repo, &["add", "."]);
    support::git(&f.repo, &["commit", "-m", "files"]);
    support::write(
        f.repo.join("中文 file.ts"),
        "export const after = '中文';\n",
    );
    support::write(f.repo.join("second.rs"), "fn after() {}\n");
    let index = std::fs::read(f.repo.join(".git/index")).unwrap();
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    let rx = capture(&mut s);
    command(&mut s, "repo diff");
    wait(&mut s);
    let Mode::Changes(session) = &s.mode else {
        panic!("changes expected")
    };
    let path = session.files.rows()[session.files.cursor]
        .location
        .name()
        .to_string_lossy()
        .into_owned();
    local_copy(&mut s);
    assert_eq!(copied(&mut s, &rx), path);
    key(&mut s, KeyCode::Char('/'));
    s.paste("copy");
    // 合法的新快照可能在编辑期间改变当前行，复制仍使用原草稿中的选择。
    let Mode::Changes(session) = &mut s.mode else {
        panic!("changes expected")
    };
    session.files.cursor = 1;
    key(&mut s, KeyCode::Enter);
    assert_eq!(copied(&mut s, &rx), path);
    let Mode::Changes(session) = &mut s.mode else {
        panic!("changes expected")
    };
    session.files.cursor = 0;
    key(&mut s, KeyCode::Char(' '));
    key(&mut s, KeyCode::Down);
    key(&mut s, KeyCode::Char(' '));
    let Mode::Changes(session) = &s.mode else {
        panic!("changes expected")
    };
    let expected = session
        .files
        .selection()
        .iter()
        .map(|c| c.location.name().to_str().unwrap())
        .collect::<Vec<_>>()
        .join("\n");
    local_copy(&mut s);
    assert_eq!(copied(&mut s, &rx), expected);
    wait(&mut s);
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    let Mode::Changes(session) = &s.mode else {
        panic!("changes expected")
    };
    assert_eq!(session.focus, crate::changes::Focus::Preview);
    let path = session.files.rows()[session.files.cursor]
        .location
        .name()
        .to_str()
        .unwrap()
        .to_owned();
    let expected = support::git(
        &f.repo,
        &["diff", "--no-ext-diff", "--no-color", "--", &path],
    );
    for layout in ["split", "unified", "auto"] {
        command(&mut s, &format!("repo diff layout {layout}"));
        key(&mut s, KeyCode::Char('/'));
        s.paste("copy");
        let text = render(&mut s, 60, 18);
        assert!(text.contains(i18n::tr!(CopyDiff)), "{text}");
        support::write(
            f.repo.join(&path),
            "external change after the visible snapshot\n",
        );
        key(&mut s, KeyCode::Enter);
        let patch = copied(&mut s, &rx);
        assert_eq!(comparable_patch(&patch), comparable_patch(&expected));
        assert_patch_applies(&f.repo, &patch);
    }
    assert_eq!(std::fs::read(f.repo.join(".git/index")).unwrap(), index);
    assert!(s.service.store.report().unwrap().is_none());
}

#[test]
fn history_and_branch_copy_follow_the_selected_object_and_historical_file() {
    let f = support::Fixture::create();
    support::write(f.repo.join("history 中文.rs"), "fn before() {}\n");
    support::git(&f.repo, &["add", "."]);
    support::git(&f.repo, &["commit", "-m", "before"]);
    support::write(f.repo.join("history 中文.rs"), "fn after() {}\n");
    support::git(&f.repo, &["commit", "-am", "after"]);
    let oid = support::git(&f.repo, &["rev-parse", "HEAD"]);
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    let rx = capture(&mut s);
    command(&mut s, "repo log");
    wait(&mut s);
    key(&mut s, KeyCode::Char('/'));
    s.paste("copy");
    support::git(&f.repo, &["commit", "--allow-empty", "-m", "new head"]);
    key(&mut s, KeyCode::Enter);
    assert_eq!(copied(&mut s, &rx), oid);
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    local_copy(&mut s);
    assert_eq!(copied(&mut s, &rx), "history 中文.rs");
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    let Mode::History(view) = &s.mode else {
        panic!("history expected")
    };
    assert!(matches!(view.as_ref(), crate::history::View::Diff { .. }));
    let expected = support::git(
        &f.repo,
        &["diff", "HEAD~2", "HEAD~1", "--", "history 中文.rs"],
    );
    local_copy(&mut s);
    let patch = copied(&mut s, &rx);
    assert_eq!(comparable_patch(&patch), comparable_patch(&expected));
    command(&mut s, "repo branch");
    wait(&mut s);
    let Mode::Branches(view) = &s.mode else {
        panic!("branches expected")
    };
    let name = view.session().selected().unwrap().name.clone();
    local_copy(&mut s);
    assert_eq!(copied(&mut s, &rx), name);
    key(&mut s, KeyCode::Enter);
    local_copy(&mut s);
    assert_eq!(copied(&mut s, &rx), name);
}

#[test]
fn errors_have_their_own_context_and_copy_complete_details_without_git_or_config_actions() {
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let (_home, mut s) = fixture(vec!["/example/repo".into()]);
        let rx = capture(&mut s);
        let config = std::fs::read(s.service.store.config_path()).unwrap();
        let detail = format!("first line\n{}\nlast line", "中文 error ".repeat(100));
        s.mode = Mode::Error {
            origin: Box::new(Mode::Browsing),
            title: "Failure".into(),
            detail: detail.clone(),
            scroll: 0,
        };
        key(&mut s, KeyCode::Char('/'));
        assert_eq!(
            s.bar.command_input().unwrap().context(),
            Some(command::Context::Error)
        );
        s.paste("copy");
        for (w, h) in [(60, 18), (100, 28), (180, 36)] {
            let text = render(&mut s, w, h);
            assert!(text.contains("/error") && text.contains("copy"), "{text}");
        }
        key(&mut s, KeyCode::Enter);
        assert_eq!(copied(&mut s, &rx), detail);
        assert!(matches!(s.mode, Mode::Error { .. }));
        key(&mut s, KeyCode::Char('/'));
        key(&mut s, KeyCode::Char('/'));
        s.paste("schedule interval 1h");
        key(&mut s, KeyCode::Enter);
        assert!(s.search_commands().unwrap().is_empty());
        assert!(matches!(s.mode, Mode::Error { .. }));
        key(&mut s, KeyCode::Esc);
        assert!(matches!(s.mode, Mode::Error { .. }));
        key(&mut s, KeyCode::Esc);
        assert!(matches!(s.mode, Mode::Browsing));
        assert_eq!(
            std::fs::read(s.service.store.config_path()).unwrap(),
            config
        );
        assert!(s.repository_tasks.is_empty());
    }
}

#[test]
fn clipboard_worker_keeps_navigation_responsive_and_reports_failures_without_overwriting_drafts() {
    let (_home, mut s) = fixture(vec!["/example/a".into(), "/example/b".into()]);
    let (started, rx) = mpsc::channel();
    let (release, gate) = mpsc::channel();
    s.clipboard = Writer::with_sink(move |text| {
        started.send(text)?;
        gate.recv_timeout(Duration::from_secs(8))?;
        anyhow::bail!("clipboard denied")
    });
    command(&mut s, "repo copy");
    assert_eq!(
        rx.recv_timeout(Duration::from_secs(8)).unwrap(),
        "/example/a"
    );
    key(&mut s, KeyCode::Down);
    assert_eq!(s.selected_path(), Some("/example/b".into()));
    command(&mut s, "repo copy");
    assert!(s.message.contains(i18n::tr!(ClipboardBusy)));
    s.paste("//repo terminal");
    let draft = s.bar.text();
    release.send(()).unwrap();
    let start = Instant::now();
    while !s.poll_clipboard() {
        assert!(start.elapsed() < Duration::from_secs(8));
        thread::sleep(Duration::from_millis(2));
    }
    assert!(s.message.contains("clipboard denied") && s.message.contains(i18n::tr!(CopyFailed)));
    assert_eq!(s.bar.text(), draft);
    assert!(s.repository_tasks.is_empty());
}

#[test]
fn unavailable_previews_never_copy_stale_or_partial_data() {
    let f = support::Fixture::create();
    support::write(f.repo.join("draft.rs"), "fn draft() {}\n");
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    let rx = capture(&mut s);
    command(&mut s, "repo diff");
    wait(&mut s);
    key(&mut s, KeyCode::Enter);
    let document = engine::local::files::patch::Document::Patch {
        files: vec![],
        extent: engine::local::files::patch::Extent::Truncated,
    };
    let content =
        document::prepare("draft.rs", document, Area::Unstaged, &Cancel::default()).unwrap();
    let Mode::Changes(session) = &mut s.mode else {
        panic!("changes expected")
    };
    let crate::changes::Preview::Ready {
        content: current, ..
    } = &mut session.preview
    else {
        panic!("preview expected")
    };
    *current = content;
    local_copy(&mut s);
    assert!(s.message.contains(i18n::tr!(CopyDiffTruncated)));
    assert!(rx.try_recv().is_err());
    let source = s.copy_selection();
    assert!(source.payload(Scope::Diff, None).is_err());
}

#[test]
fn image_file_paths_are_copyable_while_the_image_preview_reports_no_text_patch() {
    let f = support::Fixture::create();
    let mut bytes = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(image::RgbaImage::new(2, 2))
        .write_to(&mut bytes, image::ImageFormat::Png)
        .unwrap();
    std::fs::write(f.repo.join("中文 image.png"), bytes.into_inner()).unwrap();
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    let rx = capture(&mut s);
    command(&mut s, "repo diff");
    wait(&mut s);
    local_copy(&mut s);
    assert_eq!(copied(&mut s, &rx), "中文 image.png");
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    local_copy(&mut s);
    assert!(s.message.contains(i18n::tr!(CopyDiffImage)));
    assert!(rx.try_recv().is_err());
    key(&mut s, KeyCode::Esc);
    local_copy(&mut s);
    assert_eq!(copied(&mut s, &rx), "中文 image.png");
}

#[test]
fn file_paths_deduplicate_the_same_marked_file_across_staged_and_unstaged_groups() {
    let f = support::Fixture::create();
    support::write(f.repo.join("same.rs"), "fn before() {}\n");
    support::git(&f.repo, &["add", "."]);
    support::git(&f.repo, &["commit", "-m", "before"]);
    support::write(f.repo.join("same.rs"), "fn staged() {}\n");
    support::git(&f.repo, &["add", "same.rs"]);
    support::write(f.repo.join("same.rs"), "fn unstaged() {}\n");
    let index = std::fs::read(f.repo.join(".git/index")).unwrap();
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    let rx = capture(&mut s);
    command(&mut s, "repo diff");
    wait(&mut s);
    s.key(
        KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL),
        &Cancel::default(),
    )
    .unwrap();
    let Mode::Changes(session) = &s.mode else {
        panic!("changes expected")
    };
    assert_eq!(session.files.selected.len(), 2);
    local_copy(&mut s);
    assert_eq!(copied(&mut s, &rx), "same.rs");
    assert_eq!(std::fs::read(f.repo.join(".git/index")).unwrap(), index);
}

#[test]
fn combined_panes_copy_the_focused_selection_and_keep_loading_or_failure_explicit() {
    use crate::workspace::{Panel, Section};
    let f = support::Fixture::create();
    support::write(f.repo.join("pending.txt"), "pending\n");
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    let rx = capture(&mut s);
    key(&mut s, KeyCode::Enter);
    wait_workspace(&mut s);
    for section in [Section::Worktree, Section::Branches, Section::History] {
        s.switch_repository_section(section);
        wait(&mut s);
        let expected = match &s.mode {
            Mode::Files(_) => "pending.txt".to_owned(),
            Mode::Branches(view) => view.session().selected().unwrap().name.clone(),
            Mode::History(view) => view.session().selected().unwrap().oid.to_string(),
            mode => panic!("ready pane expected: {mode:?}"),
        };
        local_copy(&mut s);
        assert_eq!(copied(&mut s, &rx), expected);
    }
    s.workspace.session_mut().unwrap().files = Panel::Loading;
    s.mode = Mode::Panel(Section::Worktree);
    s.sync_bar();
    key(&mut s, KeyCode::Char('/'));
    s.paste("copy");
    s.workspace.session_mut().unwrap().files = Panel::Failed("complete panel failure\n中文".into());
    key(&mut s, KeyCode::Enter);
    assert!(s.message.contains(i18n::tr!(CopySelectionLoading)));
    assert!(rx.try_recv().is_err());
    local_copy(&mut s);
    assert_eq!(copied(&mut s, &rx), "complete panel failure\n中文");
}

#[test]
fn background_reads_cannot_replace_an_empty_scoped_command_or_its_copy_selection() {
    for succeeds in [true, false] {
        let f = support::Fixture::create();
        support::write(f.repo.join("pending.rs"), "fn pending() {}\n");
        let (_home, mut s) = fixture(vec![f.repo.clone()]);
        let rx = capture(&mut s);
        command(&mut s, "repo diff");
        wait(&mut s);
        let path = f.repo.clone();
        let (release, gate) = mpsc::channel();
        s.start_repository(
            f.repo.clone(),
            "delayed read",
            Effect::ReadOnly,
            move |service, cancel, _| {
                gate.recv_timeout(Duration::from_secs(8)).unwrap();
                if succeeds {
                    Ok(Outcome::Opened(Box::new(crate::navigation::Content::Diff(
                        FileSession::new(service.files(&path, &cancel)?),
                    ))))
                } else {
                    anyhow::bail!("read failed after opening the command")
                }
            },
        );
        key(&mut s, KeyCode::Char('/'));
        assert!(s.bar.editor().text.is_empty());
        let expected = s.bar.text();
        release.send(()).unwrap();
        wait(&mut s);
        assert_eq!(s.bar.text(), expected);
        assert!(matches!(s.mode, Mode::Changes(_)));
        s.paste("copy");
        key(&mut s, KeyCode::Enter);
        assert_eq!(copied(&mut s, &rx), "pending.rs");
    }
}
