use super::*;

fn wait_repository(s: &mut Screen) {
    let start = Instant::now();
    loop {
        s.poll_command_repository();
        if matches!(&s.bar, bar::InputBar::Command { repository, .. } if repository.resolved()) {
            break;
        }
        assert!(start.elapsed() < Duration::from_secs(8));
        thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn scoped_common_commands_are_explicit_and_complete_to_global_without_losing_selection() {
    let f = support::Fixture::create();
    support::write(f.repo.join("中文 file.ts"), "content\n");
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    let (tx, rx) = mpsc::channel();
    s.clipboard = crate::clipboard::Writer::with_sink(move |text| {
        tx.send(text)?;
        Ok(())
    });
    command(&mut s, "repo diff");
    wait(&mut s);
    key(&mut s, KeyCode::Char('/'));
    assert_eq!(
        s.bar.command_input().unwrap().context(),
        Some(command::Context::Diff(
            command::context::DiffContext::Worktree
        ))
    );
    let names = s
        .search_commands()
        .unwrap()
        .into_iter()
        .map(|entry| entry.name)
        .collect::<Vec<_>>();
    for expected in [
        "copy",
        "refresh",
        "editor",
        "open",
        "open terminal",
        if cfg!(target_os = "macos") {
            "open finder"
        } else if cfg!(target_os = "windows") {
            "open explorer"
        } else {
            "open browse"
        },
    ] {
        assert!(names.contains(&expected));
    }
    s.paste("copy");
    let text = render(&mut s, 100, 32);
    let row = s.command_items[0].0;
    assert!(text.lines().nth(row.y as usize).unwrap().contains("/copy"));
    key(&mut s, KeyCode::Tab);
    assert_eq!(s.bar.text(), "/copy");
    assert_eq!(s.bar.command_input().unwrap().context(), None);
    key(&mut s, KeyCode::Enter);
    assert_eq!(
        rx.recv_timeout(Duration::from_secs(8)).unwrap(),
        "中文 file.ts"
    );
    assert!(matches!(s.mode, Mode::Changes(_)));
    s.shutdown();
}

#[test]
fn selected_file_hints_keep_the_leaf_name_in_narrow_command_menus() {
    let f = support::Fixture::create();
    let name = "src/very-long-directory/another-very-long-directory/main-中文.ts";
    let path = f.repo.join(name);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    f.commit(name, "content\n");
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo files");
    browser::ready(&mut s);
    s.paste("main-中文.ts");
    browser::ready(&mut s);
    s.paste("//editor");
    for width in [44, 60, 80] {
        let text = render(&mut s, width, 28);
        let row = s.command_items[0].0;
        let line = text.lines().nth(row.y as usize).unwrap();
        assert!(line.contains("main-中文.ts"), "{line}");
        assert!(line.contains('…'), "{line}");
    }
    let bar::InputBar::Command {
        path, selection, ..
    } = &s.bar
    else {
        panic!("command")
    };
    assert_eq!(
        selection.editor_target(path.as_deref().unwrap()),
        Some(f.repo.join(name))
    );
    s.shutdown();
}

#[test]
fn repository_row_shows_the_frozen_branch_and_subcommands_keep_their_own_annotations() {
    let f = support::Fixture::create();
    support::git(&f.repo, &["branch", "-m", "feature/中文"]);
    let (_home, mut s) = fixture(vec![f.repo.clone(), "/example/other".into()]);
    let index = std::fs::read(f.repo.join(".git/index")).unwrap();
    s.focus(Some(f.repo.clone()));
    s.paste("//repo");
    wait_repository(&mut s);
    let text = render(&mut s, 150, 32);
    assert!(text.contains("feature/中文"), "{text}");
    let frozen = match &s.bar {
        bar::InputBar::Command { repository, .. } => repository.clone(),
        _ => panic!("command"),
    };
    support::git(&f.repo, &["branch", "-m", "changed-after-menu"]);
    s.focus(Some("/example/other".into()));
    s.poll_command_repository();
    let text = render(&mut s, 150, 32);
    assert!(text.contains("feature/中文"), "{text}");
    assert!(frozen.label(&f.repo).unwrap().contains("feature/中文"));
    s.paste(" push");
    assert!(!render(&mut s, 150, 32).contains("feature/中文"));
    key(&mut s, KeyCode::Esc);
    s.focus(Some(f.repo.clone()));
    s.paste("//repo");
    wait_repository(&mut s);
    assert!(render(&mut s, 150, 32).contains("changed-after-menu"));
    key(&mut s, KeyCode::Esc);
    command(&mut s, "repo log");
    wait(&mut s);
    s.paste("//repo log cherry-pick");
    wait_repository(&mut s);
    let text = render(&mut s, 150, 32);
    let row = s.command_items[0].0;
    let line = text.lines().nth(row.y as usize).unwrap();
    assert!(!line.contains("changed-after-menu"), "{line}");
    assert!(
        line.contains(&i18n::tf!(CommandTargetCommits, count = 1)),
        "{line}"
    );
    assert_eq!(std::fs::read(f.repo.join(".git/index")).unwrap(), index);
    assert!(s.repository_tasks.is_empty() && s.check_task.is_none());
    s.shutdown();
}
