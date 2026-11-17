use super::*;
use crate::{live, workspace};
mod window_focus;
// 原生 stream 的建立与销毁串行验证；跨仓库并发在同一观察器中验证。
static NATIVE: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn tick(s: &mut Screen) {
    s.poll_check();
    s.poll_live();
    s.poll_remote_info();
    s.poll_workspace();
    s.overview.poll(&s.service);
    s.poll(&Cancel::default());
    s.poll_changes();
}
#[track_caller]
fn until(s: &mut Screen, ready: impl Fn(&Screen) -> bool) {
    let callsite = std::panic::Location::caller();
    let start = Instant::now();
    loop {
        tick(s);
        if ready(s) {
            return;
        }
        assert!(
            start.elapsed() < Duration::from_secs(30),
            "local observation timed out at {callsite}: {}; {}",
            s.live.error().unwrap_or_default(),
            s.live.diagnostics()
        );
        thread::sleep(Duration::from_millis(10));
    }
}
fn observed(s: &Screen, path: &std::path::Path) -> bool {
    s.live.result(path).is_some()
}

#[test]
fn startup_observes_local_state_without_fetching_or_running_retired_steps() {
    use std::os::unix::fs::PermissionsExt;
    let _native = NATIVE.lock().unwrap();
    for start in [crate::Start::Repositories, crate::Start::CurrentDirectory] {
        let f = support::Fixture::create();
        f.commit("file.txt", "committed\n");
        support::write(f.repo.join("file.txt"), "local changes\n");
        f.advance();
        let marker = f.temp.path().join("unexpected-fetch");
        let upload = f.temp.path().join("upload-pack");
        support::write(
            &upload,
            &format!("#!/bin/sh\ntouch '{}'\nexit 99\n", marker.display()),
        );
        std::fs::set_permissions(&upload, std::fs::Permissions::from_mode(0o700)).unwrap();
        support::git(
            &f.repo,
            &[
                "config",
                "remote.origin.uploadpack",
                upload.to_str().unwrap(),
            ],
        );
        let (_home, old) = fixture(vec![f.repo.clone()]);
        let config = std::fs::read_to_string(old.service.store.config_path()).unwrap();
        let legacy = format!(
            "check_on_startup = true\nstartup_actions = ['fetch', 'pull', 'push', 'inspect']\n{config}"
        );
        std::fs::write(old.service.store.config_path(), &legacy).unwrap();
        let index = std::fs::read(f.repo.join(".git/index")).unwrap();
        let refs = support::git(&f.repo, &["show-ref"]);
        let mut s = start
            .in_directory(&f.repo, &Cancel::default())
            .unwrap()
            .open((*old.service).clone(), &Cancel::default())
            .unwrap();
        until(&mut s, |s| observed(s, &f.repo));
        assert!(
            matches!(s.result_for(&f.repo), Some(RepositoryResult::Attention { issues, .. }) if issues.iter().any(|issue| issue.kind == IssueKind::Dirty))
        );
        assert!(s.check_task.is_none());
        assert!(!marker.exists());
        assert_eq!(support::git(&f.repo, &["show-ref"]), refs);
        assert_eq!(std::fs::read(f.repo.join(".git/index")).unwrap(), index);
        assert_eq!(
            std::fs::read_to_string(f.repo.join("file.txt")).unwrap(),
            "local changes\n"
        );
        assert_eq!(
            std::fs::read_to_string(s.service.store.config_path()).unwrap(),
            legacy
        );
        assert!(s.service.store.tasks(10).unwrap().is_empty());
        assert!(s.service.store.report().unwrap().is_none());
        s.shutdown();
    }
}

#[test]
fn external_edits_staging_and_commits_refresh_without_navigation_or_remote_checks() {
    let _native = NATIVE.lock().unwrap();
    let f = support::Fixture::create();
    f.commit("中文 文件.ts", "export const value = 1;\n");
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    key(&mut s, KeyCode::Enter);
    wait_workspace(&mut s);
    overview::wait_overview(&mut s);
    s.switch_repository_section(workspace::Section::Worktree);
    s.live = live::State::start();
    until(&mut s, |s| observed(s, &f.repo));
    let report = s.service.store.report().unwrap();
    let config = std::fs::read(s.service.store.config_path()).unwrap();
    let refs = support::git(&f.repo, &["show-ref"]);
    support::write(f.repo.join("中文 文件.ts.tmp"), "export const value = 2;\n");
    std::fs::rename(f.repo.join("中文 文件.ts.tmp"), f.repo.join("中文 文件.ts")).unwrap();
    until(
        &mut s,
        |s| matches!(&s.mode, Mode::Files(files) if files.snapshot.changes.len() == 1),
    );
    let Mode::Files(files) = &mut s.mode else {
        panic!("files");
    };
    files.toggle_current();
    files.query = "中文".into();
    files.offset = 3;
    s.paste("/repo diff sta");
    let input = s.bar.command_input().unwrap().text().to_owned();
    support::git(&f.repo, &["add", "中文 文件.ts"]);
    until(
        &mut s,
        |s| matches!(&s.mode, Mode::Files(files) if files.snapshot.changes.iter().any(|change| change.area == Area::Staged)),
    );
    assert_eq!(s.bar.command_input().unwrap().text(), input);
    assert!(matches!(&s.mode, Mode::Files(files) if files.query == "中文" && files.offset == 3));
    let index = std::fs::read(f.repo.join(".git/index")).unwrap();
    for _ in 0..30 {
        tick(&mut s);
        thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(std::fs::read(f.repo.join(".git/index")).unwrap(), index);
    assert_eq!(support::git(&f.repo, &["show-ref"]), refs);
    support::git(&f.repo, &["commit", "-m", "external commit"]);
    until(
        &mut s,
        |s| matches!(&s.mode, Mode::Files(files) if files.snapshot.changes.is_empty()),
    );
    until(
        &mut s,
        |s| matches!(&s.workspace.session().unwrap().history, workspace::Panel::Ready(graph) if graph.records[0].entry.subject == "external commit"),
    );
    assert_eq!(s.bar.command_input().unwrap().text(), input);
    assert!(report.is_none());
    assert!(s.service.store.report().unwrap().is_none());
    assert!(s.service.store.repository_reports().unwrap().is_empty());
    assert_eq!(
        std::fs::read(s.service.store.config_path()).unwrap(),
        config
    );
    assert!(s.tasks.records.is_empty());
    s.shutdown();
}

#[test]
fn native_refresh_updates_other_repositories_and_keeps_remote_failure_and_timestamp() {
    let _native = NATIVE.lock().unwrap();
    let a = support::Fixture::create();
    let b = support::Fixture::create();
    a.commit("file", "before");
    b.commit("file", "before");
    let (_home, mut s) = fixture(vec![a.repo.clone(), b.repo.clone()]);
    let at = chrono::DateTime::parse_from_rfc3339("2026-01-01T00:00:00+00:00").unwrap();
    s.merge_update(
        a.repo.clone(),
        crate::checks::Update {
            result: RepositoryResult::Failed {
                path: a.repo.clone(),
                error: "remote unavailable".into(),
                pushes: vec![],
            },
            finished_at: at,
            revision: 3,
        },
    );
    s.live = live::State::start();
    until(&mut s, |s| observed(s, &a.repo) && observed(s, &b.repo));
    s.focus(Some(a.repo.clone()));
    s.paste("/help");
    support::write(b.repo.join("file"), "after");
    until(
        &mut s,
        |s| matches!(s.result_for(&b.repo), Some(RepositoryResult::Attention { issues, .. }) if issues.iter().any(|issue| issue.kind == IssueKind::Dirty)),
    );
    assert_eq!(s.selected_path(), Some(a.repo.clone()));
    assert!(
        matches!(s.result_for(&a.repo), Some(RepositoryResult::Failed { error, .. }) if error == "remote unavailable")
    );
    assert_eq!(s.observation_order(&a.repo), Some((3, at)));
    assert_eq!(s.bar.command_input().unwrap().text(), "/help");
    s.shutdown();
}

#[test]
fn linked_worktree_reference_changes_update_branch_and_keep_repository_panel_selection() {
    let _native = NATIVE.lock().unwrap();
    let f = support::Fixture::create();
    let linked = std::fs::canonicalize(f.temp.path())
        .unwrap()
        .join("linked worktree");
    support::git(
        &f.repo,
        &["worktree", "add", "-b", "linked", linked.to_str().unwrap()],
    );
    let (_home, mut s) = fixture(vec![linked.clone()]);
    key(&mut s, KeyCode::Enter);
    wait_workspace(&mut s);
    overview::wait_overview(&mut s);
    s.live = live::State::start();
    until(&mut s, |s| observed(s, &linked));
    support::git(&f.repo, &["branch", "new-external-branch"]);
    until(
        &mut s,
        |s| matches!(&s.workspace.session().unwrap().branches, workspace::Panel::Ready(branches) if branches.snapshot.entries.iter().any(|entry| entry.name == "new-external-branch")),
    );
    support::git(&linked, &["switch", "-c", "different"]);
    until(&mut s, |s| {
        s.overview
            .content
            .snapshot()
            .and_then(|snap| snap.checkout.as_ref().ok())
            .is_some_and(|checkout| format!("{checkout:?}").contains("different"))
    });
    assert_eq!(
        s.workspace.session().unwrap().selection,
        workspace::Section::Overview
    );
    assert!(matches!(s.mode, Mode::Browsing));
    s.shutdown();
}

#[test]
fn external_commit_updates_graph_while_open_commit_preview_remains_an_immutable_snapshot() {
    let _native = NATIVE.lock().unwrap();
    let f = support::Fixture::create();
    f.commit("file.ts", "export const old = 1;\n");
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo log");
    wait(&mut s);
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    let source = match &s.mode {
        Mode::History(view) => match view.as_ref() {
            crate::history::View::Commit(commit) => commit.inspect.detail.clone(),
            _ => panic!("commit preview"),
        },
        _ => panic!("history"),
    };
    s.live = live::State::start();
    until(&mut s, |s| observed(s, &f.repo));
    f.commit("file.ts", "export const new = 2;\n");
    until(
        &mut s,
        |s| matches!(&s.mode, Mode::History(view) if view.session().records[0].entry.subject == "file.ts" && view.session().records.len() == 3),
    );
    let Mode::History(view) = &s.mode else {
        panic!("history");
    };
    let crate::history::View::Commit(commit) = view.as_ref() else {
        panic!("commit preview");
    };
    match (&source, &commit.inspect.detail) {
        (crate::inspection::Source::Commit(before), crate::inspection::Source::Commit(after)) => {
            assert!(Arc::ptr_eq(before, after))
        }
        _ => panic!("commit source"),
    }
    s.shutdown();
}

#[test]
fn file_browser_keeps_preview_search_and_reading_position_across_external_save() {
    let _native = NATIVE.lock().unwrap();
    let f = support::Fixture::create();
    f.commit("file.ts", &"export const value = 1;\n".repeat(80));
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo files");
    wait(&mut s);
    browser::ready(&mut s);
    browser::select(&mut s, "file.ts");
    browser::ready(&mut s);
    s.live = live::State::start();
    until(&mut s, |s| observed(s, &f.repo));
    browser::ready(&mut s);
    browser::session(&mut s)
        .panes
        .get_mut(crate::browser::opening::TextMode::Source)
        .position
        .scroll = 23;
    support::write(
        f.repo.join("file.ts"),
        &"export const value = 42;\n".repeat(80),
    );
    until(
        &mut s,
        |s| matches!(&s.mode, Mode::Browser(session) if matches!(session.panes.get(crate::browser::opening::TextMode::Source).preview.document(), Some(crate::browser::preview::Document::Text { text, .. }) if text.to_string().contains("42"))),
    );
    assert_eq!(browser::session(&mut s).pane().position.scroll, 23);
    assert_eq!(
        browser::session(&mut s).selected().unwrap().path,
        std::path::Path::new("file.ts")
    );
    s.shutdown();
}

#[test]
fn remote_config_changes_refresh_the_visible_repository_details() {
    let _native = NATIVE.lock().unwrap();
    let f = support::Fixture::create();
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    s.live = live::State::start();
    until(&mut s, |s| observed(s, &f.repo));
    until(&mut s, |s| {
        matches!(
            s.remote_info.content,
            crate::remote_info::Content::Ready { .. }
        )
    });
    let url = "https://example.com/team/renamed-project.git";
    support::git(&f.repo, &["remote", "set-url", "origin", url]);
    until(&mut s, |s| {
        matches!(&s.remote_info.content,
        crate::remote_info::Content::Ready { result: Ok(endpoints), .. }
            if endpoints.iter().any(|endpoint| endpoint.url == url))
    });
    let text = render(&mut s, 180, 40);
    assert!(text.contains(url), "{text}");
    assert!(s.service.store.tasks(10).unwrap().is_empty());
    s.shutdown();
}
