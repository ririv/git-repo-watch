use super::*;
use crate::command::{
    completion::Menu,
    loader::{Output, Request},
    values::{Choice, Kind},
};
use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};

fn values(s: &Screen) -> &command::values::Candidates {
    let bar::InputBar::Command {
        menu: Menu::Values(values),
        ..
    } = &s.bar
    else {
        panic!("repository parameter candidates expected");
    };
    values
}

#[test]
fn groups_put_direct_entries_before_deeper_actions_without_hiding_descendants() {
    let commands = command::matches("repo");
    for name in [
        "repo diff",
        "repo log",
        "repo push",
        "repo sync",
        "repo files",
    ] {
        let entry = commands
            .iter()
            .position(|command| command.name == name)
            .unwrap();
        let nested = commands
            .iter()
            .position(|command| command.name == "repo diff stage")
            .unwrap();
        assert!(entry < nested, "{name}");
    }
    assert_eq!(commands[0].name, "repo");
    assert!(
        commands
            .iter()
            .any(|command| command.name == "repo diff stage")
    );
    let scoped = command::Context::Repository
        .matches("")
        .into_iter()
        .filter(|command| !command.common())
        .collect::<Vec<_>>();
    let direct = scoped
        .iter()
        .take_while(|command| command.name.split_whitespace().count() == 2)
        .count();
    assert!(direct > 8);
    assert!(
        scoped[direct..]
            .iter()
            .all(|command| command.name.split_whitespace().count() > 2)
    );
    assert_eq!(
        command::matches("repo branch sw")[0].name,
        "repo branch switch"
    );
}

#[test]
fn branch_parameters_filter_local_names_complete_on_tab_and_keep_the_captured_repository() {
    for language in [i18n::Language::Chinese, i18n::Language::English] {
        let _language = i18n::using(language);
        let first = support::Fixture::create();
        let second = support::Fixture::create();
        first.commit("name-only.txt", "branch names\n");
        support::git(&first.repo, &["branch", "feature/中文"]);
        let blob = support::git(&first.repo, &["rev-parse", "HEAD:name-only.txt"]);
        support::git(
            &first.repo,
            &[
                "update-ref",
                "refs/remotes/origin/not-a-commit",
                blob.trim(),
            ],
        );
        support::git(
            &first.repo,
            &["update-ref", "refs/remotes/origin/remote-only", "HEAD"],
        );
        let before = support::git(&first.repo, &["symbolic-ref", "HEAD"]);
        let (_home, mut s) = fixture(vec![first.repo.clone(), second.repo.clone()]);
        s.focus(Some(first.repo.clone()));
        s.paste("//repo branch switch ");
        s.focus(Some(second.repo.clone()));
        wait(&mut s);
        assert!(
            values(&s)
                .entries()
                .any(|entry| entry.token == "feature/中文")
        );
        assert!(
            !values(&s)
                .entries()
                .any(|entry| entry.token.contains("remote-only"))
        );
        s.paste("中文");
        assert_eq!(values(&s).options.len(), 1);
        key(&mut s, KeyCode::Tab);
        assert_eq!(s.bar.text(), "/repo branch switch feature/中文");
        assert!(matches!(s.mode, Mode::Browsing));
        assert_eq!(support::git(&first.repo, &["symbolic-ref", "HEAD"]), before);
        key(&mut s, KeyCode::Enter);
        wait(&mut s);
        assert!(
            matches!(&s.mode, Mode::Confirm { operation: Confirmation::Mutation { plan, .. }, .. } if plan.path() == first.repo)
        );
        key(&mut s, KeyCode::Esc);
        assert_eq!(s.bar.text(), "/repo branch switch feature/中文");
        assert_eq!(support::git(&first.repo, &["symbolic-ref", "HEAD"]), before);
        s.shutdown();
    }
}

#[test]
fn stash_candidates_show_messages_but_review_the_full_object_selected_by_mouse() {
    let f = support::Fixture::create();
    f.commit("stash.txt", "base\n");
    support::write(f.repo.join("stash.txt"), "first\n");
    support::git(&f.repo, &["stash", "push", "-m", "中文修复"]);
    let selected = support::git(&f.repo, &["rev-parse", "refs/stash"])
        .trim()
        .to_owned();
    support::write(f.repo.join("stash.txt"), "second\n");
    support::git(&f.repo, &["stash", "push", "-m", "another change"]);
    let before = support::git(&f.repo, &["stash", "list"]);
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    s.paste("//repo stash apply ");
    wait(&mut s);
    s.paste("中文");
    assert_eq!(values(&s).options.len(), 1);
    for (width, height) in [(0, 0), (1, 1), (20, 8)] {
        render(&mut s, width, height);
    }
    let rendered = render(&mut s, 120, 28);
    assert!(rendered.contains("stash@{1}") && rendered.contains("中文修复"));
    let (area, _) = s.command_items[0];
    s.mouse(
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: area.x,
            row: area.y,
            modifiers: KeyModifiers::NONE,
        },
        &Cancel::default(),
    )
    .unwrap();
    wait(&mut s);
    assert!(
        matches!(&s.mode, Mode::Confirm { operation: Confirmation::Mutation { plan, .. }, .. }
        if crate::mutations::review(plan).contains(&selected))
    );
    assert_eq!(support::git(&f.repo, &["stash", "list"]), before);
    assert_eq!(
        std::fs::read_to_string(f.repo.join("stash.txt")).unwrap(),
        "base\n"
    );
    key(&mut s, KeyCode::Esc);
    assert_eq!(s.bar.text(), format!("/repo stash apply {selected}"));
    s.shutdown();
}

#[test]
fn directory_candidates_refine_without_another_read_and_tab_fills_the_selected_path() {
    let (home, mut s) = fixture(vec![]);
    for name in ["目录 甲", "目录 乙"] {
        std::fs::create_dir(home.path().join(name)).unwrap();
    }
    s.paste("//repos add ~/目");
    complete_path(&mut s);
    s.paste("不存在");
    assert!(
        matches!(&s.bar, bar::InputBar::Command { menu: Menu::Directories(values), .. } if values.names().is_empty())
    );
    key(&mut s, KeyCode::Tab);
    assert!(matches!(s.mode, Mode::Browsing));
    for _ in 0..3 {
        key(&mut s, KeyCode::Backspace);
    }
    assert!(
        matches!(&s.bar, bar::InputBar::Command { menu: Menu::Directories(values), .. } if values.names().len() == 2)
    );
    assert!(!s.command_reader.pending());
    key(&mut s, KeyCode::Down);
    key(&mut s, KeyCode::Tab);
    assert!(s.bar.text().ends_with("/目录 甲/"));
    assert!(matches!(s.mode, Mode::Browsing));
    assert!(s.service.store.config().unwrap().repositories.is_empty());
    s.shutdown();
}

#[test]
fn pending_reads_filter_the_latest_query_and_cancelled_directory_results_do_not_replace_edits() {
    let (home, mut s) = fixture(vec!["/example/alpha".into()]);
    let request = Request::Values {
        kind: Kind::Branch,
        path: "/example/alpha".into(),
    };
    s.bar = bar::InputBar::Command {
        input: command::Input::global(Editor::new("/repo branch switch ".into())),
        selected: 0,
        menu: Menu::Loading(request.clone()),
        path: Some("/example/alpha".into()),
        selection: Default::default(),
        repository: Default::default(),
    };
    let (entered_tx, entered_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    s.command_reader.request_with(request, move |_| {
        entered_tx.send(()).unwrap();
        release_rx.recv().unwrap();
        Ok(Output::Values {
            kind: Kind::Branch,
            entries: vec![
                Choice {
                    token: "中文".into(),
                    label: "中文".into(),
                    description: String::new(),
                },
                Choice {
                    token: "other".into(),
                    label: "other".into(),
                    description: String::new(),
                },
            ],
        })
    });
    entered_rx.recv_timeout(Duration::from_secs(2)).unwrap();
    s.paste("中文");
    release_tx.send(()).unwrap();
    wait(&mut s);
    assert_eq!(values(&s).options.len(), 1);
    assert_eq!(values(&s).entries().next().unwrap().token, "中文");
    key(&mut s, KeyCode::Home);
    assert!(matches!(
        s.bar,
        bar::InputBar::Command {
            menu: Menu::Commands,
            ..
        }
    ));
    assert!(!s.command_reader.pending());
    let editor = Editor::new("/repos add /initial/".into());
    let request = Request::Directory {
        editor: editor.clone(),
        home: home.path().into(),
    };
    s.bar = bar::InputBar::Command {
        input: command::Input::global(editor),
        selected: 0,
        menu: Menu::Loading(request.clone()),
        path: None,
        selection: Default::default(),
        repository: Default::default(),
    };
    let (entered_tx, entered_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    s.command_reader.request_with(request, move |_| {
        entered_tx.send(()).unwrap();
        release_rx.recv().unwrap();
        Ok(Output::Directory {
            editor: Editor::new("/repos add /stale/".into()),
            menu: Menu::Commands,
        })
    });
    entered_rx.recv_timeout(Duration::from_secs(2)).unwrap();
    s.paste("edited");
    s.poll_command_completion();
    release_tx.send(()).unwrap();
    wait(&mut s);
    assert_eq!(s.bar.text(), "/repos add /initial/edited");
    assert!(matches!(s.mode, Mode::Browsing));
    assert!(s.service.store.config().unwrap().repositories.len() == 1);
    s.shutdown();
}

#[test]
fn empty_layout_commands_open_the_same_horizontal_picker_and_keep_the_draft_on_cancel() {
    for suffix in ["", " ", "   "] {
        let (_home, mut s) = fixture(vec![]);
        s.paste(&format!("//repo diff layout{suffix}"));
        key(&mut s, KeyCode::Enter);
        assert!(matches!(s.bar, bar::InputBar::Layout { .. }));
        key(&mut s, KeyCode::Esc);
        assert_eq!(s.bar.text(), format!("/repo diff layout{suffix}"));
        assert_eq!(s.diff_preference, crate::diff::Preference::Auto);
        s.shutdown();
    }
}

#[test]
fn unavailable_commands_explain_their_condition_and_file_counts_match_the_selected_scope() {
    let (_home, mut empty) = fixture(vec![]);
    for action in [
        Action::Log,
        Action::Compose(crate::composition::Action::Open),
    ] {
        assert_eq!(
            empty
                .command_availability(action, None, &Default::default())
                .reason(),
            Some(i18n::tr!(SelectARepositoryFirst))
        );
    }
    empty.shutdown();
    let f = support::Fixture::create();
    for name in ["one.txt", "two.txt", "three.txt"] {
        support::write(f.repo.join(name), "new\n");
    }
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    s.paste("//repo diff stage");
    let text = render(&mut s, 120, 28);
    assert!(text.contains(i18n::tr!(ChangesSelectionRequired)));
    key(&mut s, KeyCode::Enter);
    assert!(matches!(s.mode, Mode::Browsing));
    assert_eq!(s.bar.text(), "/repo diff stage");
    key(&mut s, KeyCode::Esc);
    open_files(&mut s);
    wait(&mut s);
    if let Mode::Files(files) = &mut s.mode {
        files.mark_file(0);
        files.mark_file(1);
    } else {
        panic!();
    }
    s.paste("//repo diff ");
    let text = render(&mut s, 150, 28);
    assert!(text.lines().any(|line| line.contains("/repo diff stage")
        && line.contains(&i18n::tf!(CommandTargetFiles, count = 2))));
    assert!(!text.contains("/repo diff unstage "), "{text}");
    let before = support::git(&f.repo, &["diff", "--cached"]);
    key(&mut s, KeyCode::Esc);
    s.paste("//repo diff unstage");
    key(&mut s, KeyCode::Enter);
    assert!(matches!(s.mode, Mode::Files(_)));
    assert_eq!(support::git(&f.repo, &["diff", "--cached"]), before);
    assert!(s.repository_tasks.is_empty());
    s.shutdown();
}
