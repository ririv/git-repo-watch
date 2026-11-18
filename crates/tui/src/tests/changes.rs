use super::*;
use crate::changes::{Focus, Preview};
use engine::local::files::IndexAction;

fn ctrl(s: &mut Screen, c: char) {
    s.key(
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL),
        &Cancel::default(),
    )
    .unwrap();
}
fn review(s: &Screen) -> &crate::changes::Session {
    let Mode::Changes(session) = &s.mode else {
        panic!("changes workspace expected: {:?}", s.mode)
    };
    session
}
fn choose(s: &mut Screen, name: &str, area: Area) {
    let Mode::Changes(session) = &mut s.mode else {
        panic!("changes workspace expected")
    };
    session.select_group(crate::files::Group::of(area));
    session.focus = Focus::Files;
    session.files.cursor = session
        .files
        .rows()
        .iter()
        .position(|change| {
            change.area == area && change.location.name() == std::path::Path::new(name)
        })
        .map(crate::files::Cursor::File)
        .unwrap();
    s.poll_changes();
    wait(s);
}

#[test]
fn command_tree_groups_index_actions_and_layout_without_mutation_or_legacy_aliases() {
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let f = support::Fixture::create();
        support::write(f.repo.join("new"), "new");
        let (_home, mut s) = fixture(vec![f.repo.clone()]);
        let index = std::fs::read(f.repo.join(".git/index")).unwrap();
        for removed in [
            "stage",
            "unstage",
            "repo diff auto",
            "repo diff split",
            "repo diff unified",
        ] {
            assert!(
                command::COMMANDS
                    .iter()
                    .all(|command| command.name != removed)
            );
        }
        assert_eq!(
            command::matches("repo diff ")
                .iter()
                .map(|command| command.name)
                .collect::<Vec<_>>(),
            [
                "repo diff stage",
                "repo diff unstage",
                "repo diff layout",
                "repo diff unstaged",
                "repo diff staged",
                "repo diff select",
                "repo diff context",
                "repo diff header",
                "repo diff discard",
                "repo diff discard-file",
                "repo diff view",
                "repo diff conflict",
                "repo diff conflict accept",
                "repo diff conflict block",
                "repo diff conflict undo",
                "repo diff conflict reset",
            ]
        );
        assert_eq!(
            command::matches("repo diff layout ")
                .iter()
                .map(|command| command.name)
                .collect::<Vec<_>>(),
            ["repo diff layout"]
        );
        command(&mut s, "repo diff stage");
        assert!(matches!(s.mode, Mode::Browsing));
        assert_eq!(s.message, i18n::tr!(ChangesSelectionRequired));
        assert!(s.repository_tasks.is_empty());
        command(&mut s, "repo diff layout");
        assert!(matches!(
            s.bar,
            crate::bar::InputBar::Command {
                menu: crate::command::completion::Menu::Options(_),
                ..
            }
        ));
        key(&mut s, KeyCode::Down);
        key(&mut s, KeyCode::Enter);
        assert_eq!(s.diff_preference, crate::diff::Preference::Split);
        assert_eq!(std::fs::read(f.repo.join(".git/index")).unwrap(), index);
        assert!(s.service.store.report().unwrap().is_none());
    }
}

#[test]
fn diff_prefix_shows_all_subcommands_and_keeps_enter_on_the_changes_entry() {
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let f = support::Fixture::create();
        support::write(f.repo.join("draft.rs"), "fn draft() {}\n");
        let (_home, mut s) = fixture(vec![f.repo.clone()]);
        let index = std::fs::read(f.repo.join(".git/index")).unwrap();
        s.paste("//di");
        key(&mut s, KeyCode::Tab);
        assert_eq!(s.bar.text(), "/repo diff");
        assert_eq!(
            s.search_commands().unwrap().len(),
            command::COMMANDS
                .iter()
                .filter(|c| c.name == "repo diff" || c.name.starts_with("repo diff "))
                .count()
        );
        let text = render(&mut s, 180, 36);
        for command in s.search_commands().unwrap().into_iter().take(8) {
            assert!(
                text.contains(command.name.split_whitespace().last().unwrap()),
                "{text}"
            );
        }
        for _ in 0..3 {
            key(&mut s, KeyCode::Down);
        }
        key(&mut s, KeyCode::Enter);
        assert!(matches!(
            s.bar,
            crate::bar::InputBar::Command {
                menu: crate::command::completion::Menu::Options(_),
                ..
            }
        ));
        key(&mut s, KeyCode::Esc);
        assert_eq!(s.bar.text(), "/repo diff layout ");
        assert!(matches!(
            s.bar,
            crate::bar::InputBar::Command { selected: 0, .. }
        ));
        key(&mut s, KeyCode::Esc);
        command(&mut s, "repo diff");
        wait(&mut s);
        assert!(matches!(s.mode, Mode::Changes(_)));
        assert!(s.repository_tasks.is_empty());
        assert_eq!(std::fs::read(f.repo.join(".git/index")).unwrap(), index);
    }
}

#[test]
fn combined_view_previews_exact_sides_and_keeps_selection_visible_after_navigation_and_resize() {
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let f = support::Fixture::create();
        f.commit("双重.ts", "export const value = 'head';\n");
        support::write(f.repo.join("双重.ts"), "export const value = 'index';\n");
        support::git(&f.repo, &["add", "双重.ts"]);
        support::write(f.repo.join("双重.ts"), "export const value = 'worktree';\n");
        support::write(f.repo.join("other.rs"), "fn other() {}\n");
        let (_home, mut s) = fixture(vec![f.repo.clone()]);
        let index = std::fs::read(f.repo.join(".git/index")).unwrap();
        command(&mut s, "repo diff");
        wait(&mut s);
        assert_eq!(review(&s).focus, Focus::Files);
        choose(&mut s, "双重.ts", Area::Unstaged);
        assert_eq!(
            review(&s).content().labels(),
            (i18n::tr!(Index).into(), i18n::tr!(Worktree).into())
        );
        let wide = render(&mut s, 180, 36);
        assert!(
            wide.contains("'index'") && wide.contains("'worktree'"),
            "{wide}"
        );
        for (width, height) in [(60, 18), (100, 28), (180, 36), (240, 40)] {
            let text = render(&mut s, width, height);
            let files_area = view::changes::areas(s.body_area.unwrap(), review(&s))
                .into_iter()
                .find(|(_, focus)| *focus == Focus::Files)
                .unwrap()
                .0;
            if files_area.height >= 7 {
                assert!(
                    text.contains(i18n::tr!(ChangesUnstaged))
                        && text.contains(i18n::tr!(ChangesStaged)),
                    "{text}"
                );
            }
            let selected = review(&s)
                .files
                .list(s.file_layout)
                .rows
                .iter()
                .find_map(|row| {
                    let selected = row.cursor().as_ref() == Some(&review(&s).files.cursor);
                    match row {
                        crate::files::list::Row::Entry { index, .. } if selected => Some(*index),
                        _ => None,
                    }
                })
                .unwrap();
            assert!((files_area.y+1..files_area.bottom()-1).any(|y| matches!(
                s.pointer.hit(ratatui::layout::Position::new(files_area.x+5,y)),
                Some(crate::pointer::Hit::Item(crate::pointer::Target::Files,index)) if index == selected
            )));
            assert!(text.contains(i18n::tr!(FileDiff)), "{text}");
        }
        choose(&mut s, "双重.ts", Area::Staged);
        assert_eq!(
            review(&s).content().labels(),
            ("HEAD".into(), i18n::tr!(Index).into())
        );
        let staged = render(&mut s, 180, 36);
        assert!(
            staged.contains("'head'")
                && staged.contains("'index'")
                && !staged.contains("'worktree'"),
            "{staged}"
        );
        key(&mut s, KeyCode::Enter);
        assert_eq!(review(&s).focus, Focus::Preview);
        zoom(&mut s);
        assert_eq!(
            review(&s).arrangement,
            crate::workspace::Arrangement::Maximized
        );
        key(&mut s, KeyCode::Esc);
        s.cycle_pane(false);
        assert_eq!(review(&s).focus, Focus::Files);
        key(&mut s, KeyCode::Esc);
        assert!(matches!(s.mode, Mode::Browsing));
        assert_eq!(s.page, Page::Repositories);
        assert_eq!(std::fs::read(f.repo.join(".git/index")).unwrap(), index);
        assert!(s.service.store.report().unwrap().is_none());
    }
}

#[test]
fn stage_and_unstage_refresh_the_same_workspace_with_group_scoped_select_all() {
    let f = support::Fixture::create();
    support::write(f.repo.join("alpha"), "alpha");
    support::write(f.repo.join("beta"), "beta");
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo diff");
    wait(&mut s);
    choose(&mut s, "alpha", Area::Untracked);
    command(&mut s, "repo diff stage");
    wait(&mut s);
    assert!(matches!(s.mode, Mode::Changes(_)));
    assert_eq!(
        support::git(&f.repo, &["diff", "--cached", "--name-only"]),
        "alpha"
    );
    assert_eq!(
        review(&s).files.current().unwrap().location.name(),
        std::path::Path::new("beta")
    );
    command(&mut s, "repo diff staged");
    wait(&mut s);
    ctrl(&mut s, 'a');
    let index = std::fs::read(f.repo.join(".git/index")).unwrap();
    command(&mut s, "repo diff stage");
    assert_eq!(s.message, i18n::tr!(CommandNeedsUnstaged));
    assert!(s.repository_tasks.is_empty());
    assert_eq!(std::fs::read(f.repo.join(".git/index")).unwrap(), index);
    key(&mut s, KeyCode::Esc);
    command(&mut s, "repo diff unstage");
    wait(&mut s);
    assert!(support::git(&f.repo, &["diff", "--cached"]).is_empty());
    assert_eq!(
        std::fs::read_to_string(f.repo.join("alpha")).unwrap(),
        "alpha"
    );
    assert_eq!(review(&s).focus, Focus::Files);
}

#[test]
fn selection_changes_coalesce_to_the_latest_file_and_refresh_recovers_stale_snapshots() {
    let f = support::Fixture::create();
    for name in ["a.rs", "b.rs", "c.rs"] {
        support::write(
            f.repo.join(name),
            &format!("fn {}() {{}}\n", name.chars().next().unwrap()),
        );
    }
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo diff");
    wait(&mut s);
    for _ in 0..30 {
        key(&mut s, KeyCode::Down);
        key(&mut s, KeyCode::Up);
    }
    key(&mut s, KeyCode::End);
    wait(&mut s);
    assert!(render(&mut s, 180, 32).contains("fn c()"));
    support::git(&f.repo, &["add", "a.rs"]);
    key(&mut s, KeyCode::Home);
    wait(&mut s);
    assert!(matches!(&review(&s).preview, Preview::Failed { .. }));
    assert!(render(&mut s, 180, 32).contains("a.rs"));
    command(&mut s, "refresh");
    wait(&mut s);
    choose(&mut s, "a.rs", Area::Staged);
    assert_eq!(
        review(&s).content().labels(),
        ("HEAD".into(), i18n::tr!(Index).into())
    );
    assert!(s.service.store.report().unwrap().is_none());
}

#[test]
fn completing_a_write_refreshes_only_files_and_preserves_a_new_command_draft() {
    let f = support::Fixture::create();
    support::write(f.repo.join("new.rs"), "fn new_file() {}\n");
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo diff");
    wait(&mut s);
    let session = review(&s).files.clone();
    let (release, gate) = mpsc::channel();
    s.start_repository(
        f.repo.clone(),
        "delayed stage",
        Effect::IndexWrite,
        move |service, cancel, _| {
            gate.recv_timeout(Duration::from_secs(8)).unwrap();
            Ok(Outcome::Files(service.edit_index(
                &session.snapshot,
                IndexAction::Stage,
                &session.selection(),
                &cancel,
            )?))
        },
    );
    s.paste("//repo diff layout sp");
    release.send(()).unwrap();
    wait(&mut s);
    assert_eq!(s.bar.text(), "/repo diff layout sp");
    assert!(review(&s).files.rows().is_empty());
    choose(&mut s, "new.rs", Area::Staged);
    assert_eq!(
        review(&s).content().labels(),
        ("HEAD".into(), i18n::tr!(Index).into())
    );
}

#[test]
fn whole_file_discard_uses_the_highlighted_worktree_row_and_requires_confirmation() {
    let f = support::Fixture::create();
    let name = "tracked.txt";
    f.commit(name, "head\n");
    support::write(f.repo.join(name), "index\n");
    support::git(&f.repo, &["add", name]);
    support::write(f.repo.join(name), "worktree\n");
    support::write(f.repo.join("untracked.txt"), "keep\n");
    let index = std::fs::read(f.repo.join(".git/index")).unwrap();
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo diff");
    wait(&mut s);
    choose(&mut s, name, Area::Staged);
    command(&mut s, "repo diff discard-file");
    assert_eq!(s.message, i18n::tr!(CommandNeedsUnstaged));
    key(&mut s, KeyCode::Esc);

    choose(&mut s, name, Area::Unstaged);
    key(&mut s, KeyCode::Enter);
    command(&mut s, "repo diff discard-file");
    assert_eq!(s.message, i18n::tr!(DiscardFileSelect));
    key(&mut s, KeyCode::Esc);
    choose(&mut s, name, Area::Unstaged);
    command(&mut s, "repo diff discard-file");
    wait(&mut s);
    assert!(matches!(
        &s.mode,
        Mode::Confirm {
            operation: Confirmation::DiscardFile { .. },
            choice: crate::confirmation::Choice::Return,
        }
    ));
    key(&mut s, KeyCode::Enter);
    assert_eq!(
        std::fs::read_to_string(f.repo.join(name)).unwrap(),
        "worktree\n"
    );
    assert_eq!(review(&s).focus, Focus::Files);

    command(&mut s, "repo diff discard-file");
    wait(&mut s);
    key(&mut s, KeyCode::Left);
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    assert_eq!(
        std::fs::read_to_string(f.repo.join(name)).unwrap(),
        "index\n"
    );
    assert_eq!(
        std::fs::read_to_string(f.repo.join("untracked.txt")).unwrap(),
        "keep\n"
    );
    assert_eq!(std::fs::read(f.repo.join(".git/index")).unwrap(), index);
    assert_eq!(review(&s).focus, Focus::Files);
    s.shutdown();
}

#[test]
fn whole_file_discard_uses_only_the_current_groups_marks() {
    let f = support::Fixture::create();
    f.commit("tracked.txt", "head\n");
    support::write(f.repo.join("tracked.txt"), "index\n");
    support::git(&f.repo, &["add", "tracked.txt"]);
    support::write(f.repo.join("tracked.txt"), "worktree\n");
    support::write(f.repo.join("delete.txt"), "delete\n");
    support::write(f.repo.join("keep.txt"), "keep\n");
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo diff");
    wait(&mut s);

    choose(&mut s, "tracked.txt", Area::Staged);
    key(&mut s, KeyCode::Insert);
    choose(&mut s, "keep.txt", Area::Untracked);
    assert_eq!(review(&s).files.marked_changes().count(), 0);
    choose(&mut s, "tracked.txt", Area::Staged);
    assert_eq!(review(&s).files.marked_changes().count(), 1);

    choose(&mut s, "delete.txt", Area::Untracked);
    key(&mut s, KeyCode::Insert);
    choose(&mut s, "tracked.txt", Area::Unstaged);
    key(&mut s, KeyCode::Insert);
    choose(&mut s, "tracked.txt", Area::Staged);
    assert_eq!(review(&s).files.marked_changes().count(), 1);
    command(&mut s, "repo diff discard-file");
    assert_eq!(s.message, i18n::tr!(CommandNeedsUnstaged));
    assert!(f.repo.join("delete.txt").exists());
    key(&mut s, KeyCode::Esc);

    choose(&mut s, "keep.txt", Area::Untracked);
    command(&mut s, "repo diff discard-file");
    wait(&mut s);

    let Mode::Confirm {
        operation: Confirmation::DiscardFile { review, .. },
        choice: crate::confirmation::Choice::Return,
    } = &s.mode
    else {
        panic!("batch confirmation expected: {:?}", s.mode);
    };
    assert!(review.contains(i18n::tr!(DiscardFileRestoreReview)));
    assert!(review.contains(i18n::tr!(DiscardFileRemoveReview)));
    assert!(review.contains("tracked.txt"));
    assert!(review.contains("delete.txt"));
    assert!(!review.contains("keep.txt"));

    key(&mut s, KeyCode::Left);
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    assert_eq!(
        std::fs::read_to_string(f.repo.join("tracked.txt")).unwrap(),
        "index\n"
    );
    assert!(!f.repo.join("delete.txt").exists());
    assert!(f.repo.join("keep.txt").exists());
    s.shutdown();
}

#[test]
fn worktree_subviews_keep_their_marks_cursor_and_scroll_when_switching() {
    use crate::{
        files::Group,
        navigation::breadcrumbs::{Destination, Level},
        pointer::{Action, Hit},
    };
    use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
    use ratatui::layout::Position;
    use std::path::Path;

    let f = support::Fixture::create();
    for prefix in ["staged", "worktree"] {
        for number in 0..32 {
            support::write(f.repo.join(format!("{prefix}-{number:02}.txt")), prefix);
        }
        if prefix == "staged" {
            support::git(&f.repo, &["add", "."]);
        }
    }
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo diff staged");
    wait(&mut s);
    assert_eq!(review(&s).active_group(), Group::Staged);
    assert_eq!(review(&s).files.rows().len(), 32);
    assert!(
        review(&s)
            .files
            .rows()
            .iter()
            .all(|change| change.area == Area::Staged)
    );
    assert!(
        s.breadcrumbs()
            .last()
            .is_some_and(|crumb| crumb.label == Group::Staged.label())
    );
    key(&mut s, KeyCode::End);
    key(&mut s, KeyCode::Insert);
    render(&mut s, 150, 26);
    let staged_offset = review(&s).files.offset;
    let history_depth = s.history.len();
    assert!(staged_offset > 0);

    let tab = (0..26)
        .flat_map(|row| (0..42).map(move |col| Position::new(col, row)))
        .find(|point| {
            s.pointer.hit(*point) == Some(Hit::Button(Action::ChangesGroup(Group::Unstaged)))
        })
        .expect("unstaged tab");
    s.mouse(
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: tab.x,
            row: tab.y,
            modifiers: KeyModifiers::NONE,
        },
        &Cancel::default(),
    )
    .unwrap();
    wait(&mut s);
    assert_eq!(review(&s).active_group(), Group::Unstaged);
    assert_eq!(review(&s).files.offset, 0);
    assert_eq!(review(&s).files.marked_changes().count(), 0);
    key(&mut s, KeyCode::Down);
    key(&mut s, KeyCode::Insert);

    command(&mut s, "repo diff staged");
    wait(&mut s);
    assert_eq!(s.history.len(), history_depth);
    assert_eq!(review(&s).files.offset, staged_offset);
    assert_eq!(
        review(&s).files.current().unwrap().location.name(),
        Path::new("staged-31.txt")
    );
    assert_eq!(
        review(&s)
            .files
            .marked_changes()
            .next()
            .unwrap()
            .location
            .name(),
        Path::new("staged-31.txt")
    );
    command(&mut s, "repo diff unstaged");
    wait(&mut s);
    assert_eq!(
        review(&s).files.current().unwrap().location.name(),
        Path::new("worktree-01.txt")
    );
    assert_eq!(
        review(&s)
            .files
            .marked_changes()
            .next()
            .unwrap()
            .location
            .name(),
        Path::new("worktree-01.txt")
    );

    s.follow_breadcrumb(Destination::Current(Level::ChangesParent));
    wait_workspace(&mut s);
    assert_eq!(s.page, Page::Detail);
    assert_eq!(s.mode.file_session().unwrap().rows().len(), 64);
    s.shutdown();
}
