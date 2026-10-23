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
fn alt(s: &mut Screen, c: char) {
    s.key(
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::ALT),
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
    session.focus = Focus::Files;
    session.files.cursor = session
        .files
        .rows()
        .iter()
        .position(|change| {
            change.area == area && change.location.name() == std::path::Path::new(name)
        })
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
                "repo diff layout auto",
                "repo diff layout split",
                "repo diff layout unified",
                "repo diff refresh",
                "repo diff copy"
            ]
        );
        assert_eq!(
            command::matches("repo diff layout ")
                .iter()
                .map(|command| command.name)
                .collect::<Vec<_>>(),
            [
                "repo diff layout auto",
                "repo diff layout split",
                "repo diff layout unified"
            ]
        );
        command(&mut s, "repo diff stage");
        assert!(matches!(s.mode, Mode::Browsing));
        assert_eq!(s.message, i18n::tr!(ChangesSelectionRequired));
        assert!(s.repository_tasks.is_empty());
        command(&mut s, "repo diff layout");
        assert!(matches!(
            s.bar,
            crate::bar::InputBar::Layout {
                selected: crate::diff::Preference::Auto,
                ..
            }
        ));
        key(&mut s, KeyCode::Right);
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
        assert_eq!(s.bar.commands().unwrap().len(), 8);
        let text = render(&mut s, 180, 36);
        for command in s.bar.commands().unwrap() {
            assert!(text.contains(&format!("/{}", command.name)), "{text}");
        }
        for _ in 0..3 {
            key(&mut s, KeyCode::Down);
        }
        key(&mut s, KeyCode::Enter);
        assert!(matches!(s.bar, crate::bar::InputBar::Layout { .. }));
        key(&mut s, KeyCode::Esc);
        assert_eq!(s.bar.text(), "/repo diff");
        assert!(matches!(
            s.bar,
            crate::bar::InputBar::Command { selected: 3, .. }
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
fn combined_view_previews_exact_sides_and_keeps_both_groups_visible_after_navigation_and_resize() {
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
            assert!(
                text.contains(i18n::tr!(ChangesUnstaged))
                    && text.contains(i18n::tr!(ChangesStaged)),
                "{text}"
            );
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
        ctrl(&mut s, 'f');
        assert_eq!(
            review(&s).arrangement,
            crate::workspace::Arrangement::Maximized
        );
        key(&mut s, KeyCode::Esc);
        key(&mut s, KeyCode::Tab);
        assert_eq!(review(&s).focus, Focus::Files);
        key(&mut s, KeyCode::Esc);
        assert!(matches!(s.mode, Mode::Browsing));
        assert_eq!(s.page, Page::Repositories);
        assert_eq!(std::fs::read(f.repo.join(".git/index")).unwrap(), index);
        assert!(s.service.store.report().unwrap().is_none());
    }
}

#[test]
fn stage_and_unstage_refresh_the_same_workspace_and_mixed_selections_are_explicit_errors() {
    let f = support::Fixture::create();
    support::write(f.repo.join("alpha"), "alpha");
    support::write(f.repo.join("beta"), "beta");
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo diff");
    wait(&mut s);
    choose(&mut s, "alpha", Area::Untracked);
    alt(&mut s, 's');
    wait(&mut s);
    assert!(matches!(s.mode, Mode::Changes(_)));
    assert_eq!(
        support::git(&f.repo, &["diff", "--cached", "--name-only"]),
        "alpha"
    );
    assert_eq!(
        review(&s).files.rows()[review(&s).files.cursor]
            .location
            .name(),
        std::path::Path::new("alpha")
    );
    ctrl(&mut s, 'a');
    let index = std::fs::read(f.repo.join(".git/index")).unwrap();
    command(&mut s, "repo diff stage");
    assert_eq!(s.message, i18n::tr!(ChangesWrongArea));
    assert!(s.repository_tasks.is_empty());
    assert_eq!(std::fs::read(f.repo.join(".git/index")).unwrap(), index);
    ctrl(&mut s, 'a');
    alt(&mut s, 'u');
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
    command(&mut s, "repo refresh");
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
    assert_eq!(review(&s).files.rows()[0].area, Area::Staged);
    assert_eq!(
        review(&s).content().labels(),
        ("HEAD".into(), i18n::tr!(Index).into())
    );
}
