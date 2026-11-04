use super::*;

#[test]
fn terminal_uses_captured_directories_and_file_parents_without_changing_selection() {
    let f = support::Fixture::create();
    std::fs::create_dir(f.repo.join("中文 目录")).unwrap();
    f.commit("中文 目录/file.ts", "content\n");
    f.commit("root.txt", "root\n");
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    assert_eq!(
        s.copy_selection().terminal_target(&f.repo),
        Some(f.repo.clone())
    );
    command(&mut s, "repo files");
    browser::ready(&mut s);
    browser::select(&mut s, "中文 目录");
    s.paste("//terminal");
    browser::select(&mut s, "root.txt");
    let bar::InputBar::Command {
        path, selection, ..
    } = &s.bar
    else {
        panic!("command")
    };
    assert_eq!(
        selection.terminal_target(path.as_deref().unwrap()),
        Some(f.repo.join("中文 目录"))
    );
    assert_eq!(
        s.command_target(Action::Terminal, path.as_deref(), selection)
            .as_deref(),
        Some("中文 目录")
    );
    key(&mut s, KeyCode::Esc);
    browser::select(&mut s, "中文 目录");
    key(&mut s, KeyCode::Enter);
    browser::ready(&mut s);
    browser::select(&mut s, "中文 目录/file.ts");
    assert_eq!(
        s.copy_selection().terminal_target(&f.repo),
        Some(f.repo.join("中文 目录"))
    );
    command(&mut s, "settings");
    assert!(
        s.command_availability(Action::Terminal, Some(&f.repo), &s.copy_selection())
            .reason()
            .is_some()
    );
    assert!(
        s.command_availability(Action::Copy, Some(&f.repo), &s.copy_selection())
            .reason()
            .is_some()
    );
    s.shutdown();
}

#[test]
fn refresh_rereads_local_files_and_preserves_branch_details_without_fetching() {
    let f = support::Fixture::create();
    f.commit("file.ts", "before\n");
    support::git(
        &f.repo,
        &["remote", "set-url", "origin", "/missing/offline-remote"],
    );
    support::write(
        f.repo.join(".git/FETCH_HEAD"),
        "unchanged remote observation\n",
    );
    let index = std::fs::read(f.repo.join(".git/index")).unwrap();
    let head = support::git(&f.repo, &["rev-parse", "HEAD"]);
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo files");
    browser::ready(&mut s);
    browser::select(&mut s, "file.ts");
    support::write(f.repo.join("file.ts"), "after\n");
    command(&mut s, "refresh");
    browser::ready(&mut s);
    let displayed = browser::text(&mut s);
    assert!(displayed.contains("after"));
    assert!(!displayed.contains("before"));
    command(&mut s, "repo branch");
    wait(&mut s);
    key(&mut s, KeyCode::Enter);
    assert!(
        matches!(&s.mode, Mode::Branches(view) if matches!(view.as_ref(), crate::branches::View::Detail { .. }))
    );
    support::git(&f.repo, &["branch", "new-local-branch"]);
    command(&mut s, "refresh");
    wait(&mut s);
    assert!(
        matches!(&s.mode, Mode::Branches(view) if matches!(view.as_ref(), crate::branches::View::Detail { .. }) && view.session().snapshot.entries.iter().any(|entry| entry.name == "new-local-branch"))
    );
    assert_eq!(std::fs::read(f.repo.join(".git/index")).unwrap(), index);
    assert_eq!(support::git(&f.repo, &["rev-parse", "HEAD"]), head);
    assert_eq!(
        std::fs::read_to_string(f.repo.join(".git/FETCH_HEAD")).unwrap(),
        "unchanged remote observation\n"
    );
    assert!(s.check_task.is_none());
    assert!(s.service.store.report().unwrap().is_none());
    s.shutdown();
}

#[test]
fn refresh_keeps_historical_snapshots_and_the_command_draft_in_place() {
    let f = support::Fixture::create();
    f.commit("file", "before\n");
    f.commit("file", "after\n");
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo log");
    wait(&mut s);
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    s.paste("//refresh");
    assert_eq!(
        s.command_availability(Action::Refresh, Some(&f.repo), &s.copy_selection())
            .reason(),
        Some(i18n::tr!(RefreshHistorical))
    );
    let draft = s.bar.text();
    key(&mut s, KeyCode::Enter);
    assert_eq!(s.bar.text(), draft);
    assert!(
        matches!(&s.mode, Mode::History(view) if matches!(view.as_ref(), crate::history::View::Diff { .. }))
    );
    assert!(s.repository_tasks.is_empty());
    s.shutdown();
}
