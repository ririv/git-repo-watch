use super::*;

#[test]
fn terminal_uses_captured_directories_and_file_parents_without_changing_selection() {
    let f = support::Fixture::create();
    std::fs::create_dir(f.repo.join("中文 目录")).unwrap();
    f.commit("中文 目录/file.ts", "content\n");
    f.commit("root.txt", "root\n");
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    assert_eq!(
        s.copy_selection().directory_target(&f.repo),
        Some(f.repo.clone())
    );
    command(&mut s, "repo files");
    browser::ready(&mut s);
    browser::select(&mut s, "中文 目录");
    s.paste("//open terminal");
    browser::select(&mut s, "root.txt");
    let bar::InputBar::Command {
        path, selection, ..
    } = &s.bar
    else {
        panic!("command")
    };
    assert_eq!(
        selection.directory_target(path.as_deref().unwrap()),
        Some(f.repo.join("中文 目录"))
    );
    assert_eq!(
        s.resolve_command(Action::Terminal, path.as_deref(), selection)
            .value()
            .and_then(|target| target.label(Action::Terminal, selection))
            .as_deref(),
        Some("中文 目录")
    );
    key(&mut s, KeyCode::Esc);
    browser::select(&mut s, "中文 目录");
    key(&mut s, KeyCode::Enter);
    browser::ready(&mut s);
    browser::select(&mut s, "中文 目录/file.ts");
    assert_eq!(
        s.copy_selection().directory_target(&f.repo),
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
fn folder_opens_the_directory_captured_by_the_group() {
    let f = support::Fixture::create();
    let folder = f.repo.join("中文 目录");
    std::fs::create_dir(&folder).unwrap();
    f.commit("中文 目录/file.ts", "content\n");
    f.commit("root.txt", "root\n");
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    let (tx, rx) = std::sync::mpsc::channel();
    s.link_opener = crate::links::Opener::with_sink(move |target, _| {
        tx.send(target.clone()).unwrap();
        Ok(())
    });

    let (file_manager, label) = if cfg!(target_os = "macos") {
        ("finder", "在访达中打开")
    } else if cfg!(target_os = "windows") {
        ("explorer", "在文件资源管理器中打开")
    } else {
        ("browse", "在文件管理器中打开")
    };
    s.paste("//open");
    key(&mut s, KeyCode::Tab);
    assert_eq!(s.bar.text(), "/open ");
    assert_eq!(
        s.search_commands()
            .unwrap()
            .iter()
            .map(|command| command.name)
            .collect::<Vec<_>>(),
        ["open terminal", format!("open {file_manager}").as_str()],
    );
    {
        let _language = i18n::using(i18n::Language::Chinese);
        let menu = render(&mut s, 120, 28);
        assert!(
            menu.lines()
                .any(|line| line.contains(file_manager) && line.contains(label)),
            "{menu}"
        );
    }
    key(&mut s, KeyCode::Down);
    key(&mut s, KeyCode::Enter);
    assert_eq!(
        rx.recv_timeout(Duration::from_secs(3)).unwrap(),
        engine::desktop::Target::directory(f.repo.clone()).unwrap(),
    );
    links::opened(&mut s);
    assert_eq!(s.message, i18n::tr!(FolderOpenRequested));

    command(&mut s, "repo files");
    browser::ready(&mut s);
    browser::select(&mut s, "中文 目录");
    key(&mut s, KeyCode::Enter);
    browser::ready(&mut s);
    browser::select(&mut s, "中文 目录/file.ts");
    key(&mut s, KeyCode::Char('/'));
    s.paste("open");
    key(&mut s, KeyCode::Enter);
    assert_eq!(s.bar.text(), "/open ");
    key(&mut s, KeyCode::Down);
    key(&mut s, KeyCode::Enter);
    assert_eq!(
        rx.recv_timeout(Duration::from_secs(3)).unwrap(),
        engine::desktop::Target::directory(folder).unwrap(),
    );
    links::opened(&mut s);
    assert_eq!(s.message, i18n::tr!(FolderOpenRequested));
    assert!(matches!(s.mode, Mode::Browser(_)));
    assert!(s.repository_tasks.is_empty());
    s.shutdown();
}

#[test]
fn refresh_rereads_local_files_and_preserves_branch_details_without_fetching() {
    let f = support::Fixture::create();
    f.commit("file.ts", "before\n");
    support::git(
        &f.repo,
        &[
            "remote",
            "set-url",
            "origin",
            fixture_path!("/missing/offline-remote"),
        ],
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
