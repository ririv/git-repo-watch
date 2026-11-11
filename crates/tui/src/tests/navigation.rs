use super::*;

#[test]
fn empty_input_arrows_open_details_and_return_without_losing_selection_or_history() {
    let (_home, mut s) = fixture(vec!["/example/alpha".into(), "/example/beta".into()]);
    key(&mut s, KeyCode::Down);
    key(&mut s, KeyCode::Right);
    assert_eq!(s.page, Page::Detail);
    assert_eq!(s.selected(), 1);
    assert_eq!(s.history.len(), 1);
    key(&mut s, KeyCode::F(1));
    assert_eq!(s.page, Page::Help);
    key(&mut s, KeyCode::Left);
    assert_eq!(s.page, Page::Detail);
    key(&mut s, KeyCode::Esc);
    assert_eq!(s.page, Page::Repositories);
    assert_eq!(s.selected(), 1);
    assert!(s.history.is_empty());
    let (_home, mut empty) = fixture(vec![]);
    key(&mut empty, KeyCode::Right);
    key(&mut empty, KeyCode::Left);
    assert_eq!(empty.page, Page::Repositories);
    assert!(matches!(empty.mode, Mode::Browsing));
}

#[test]
fn empty_input_file_arrows_open_diff_and_return_through_the_same_file_session() {
    let f = support::Fixture::create();
    support::write(f.repo.join("navigation-file"), "navigation content");
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    key(&mut s, KeyCode::Right);
    open_files(&mut s);
    wait(&mut s);
    assert!(matches!(s.mode, Mode::Files(_)));
    key(&mut s, KeyCode::Char(' '));
    key(&mut s, KeyCode::Right);
    wait(&mut s);
    assert!(matches!(&s.mode, Mode::Changes(_)));
    assert!(render(&mut s, 100, 28).contains("navigation content"));
    key(&mut s, KeyCode::Left);
    assert!(
        matches!(&s.mode, Mode::Files(session) if session.cursor == crate::files::Cursor::File(0) && session.selected.len() == 1)
    );
    key(&mut s, KeyCode::Left);
    assert!(matches!(s.mode, Mode::Browsing));
    assert_eq!(s.page, Page::Detail);
    key(&mut s, KeyCode::Esc);
    assert_eq!(s.page, Page::Repositories);
    assert!(support::git(&f.repo, &["diff", "--cached"]).is_empty());
    assert!(s.service.store.report().unwrap().is_none());
}

#[test]
fn text_and_modified_arrows_stay_in_the_editor_instead_of_navigating() {
    let (_home, mut s) = fixture(vec!["/example/中文 alpha".into()]);
    s.paste("中文");
    let end = s.bar.editor().cursor;
    key(&mut s, KeyCode::Left);
    assert_eq!(s.bar.editor().cursor, end - '文'.len_utf8());
    key(&mut s, KeyCode::Right);
    key(&mut s, KeyCode::Right);
    assert_eq!(s.bar.editor().cursor, end);
    assert_eq!(s.page, Page::Repositories);
    assert_eq!(s.query, "中文");
    key(&mut s, KeyCode::Esc);
    for modifier in [
        KeyModifiers::CONTROL,
        KeyModifiers::ALT,
        KeyModifiers::SHIFT,
    ] {
        s.key(KeyEvent::new(KeyCode::Right, modifier), &Cancel::default())
            .unwrap();
        assert_eq!(s.page, Page::Repositories);
        assert!(matches!(s.mode, Mode::Browsing));
    }
    s.paste("//repos add ~/中文");
    let draft = s.bar.editor().text.clone();
    key(&mut s, KeyCode::Left);
    key(&mut s, KeyCode::Right);
    assert_eq!(s.bar.editor().text, draft);
    assert!(matches!(s.mode, Mode::Browsing));
    assert_eq!(s.page, Page::Repositories);
}

#[test]
fn settings_arrows_open_editing_or_confirmation_without_executing_the_operation() {
    let (_home, mut s) = fixture(vec![]);
    let before = std::fs::read(s.service.store.config_path()).unwrap();
    command(&mut s, "settings");
    key(&mut s, KeyCode::Right);
    assert_eq!(s.bar.text(), "/settings concurrency 4");
    assert!(matches!(s.mode, Mode::Browsing));
    key(&mut s, KeyCode::Esc);
    key(&mut s, KeyCode::Left);
    assert_eq!(s.page, Page::Repositories);
    command(&mut s, "schedule");
    key(&mut s, KeyCode::Down);
    key(&mut s, KeyCode::Down);
    key(&mut s, KeyCode::Down);
    key(&mut s, KeyCode::Down);
    key(&mut s, KeyCode::Right);
    assert!(matches!(
        s.mode,
        Mode::Confirm {
            operation: Confirmation::Schedule(engine::scheduler::Control::Apply),
            ..
        }
    ));
    key(&mut s, KeyCode::Right);
    assert!(matches!(
        s.mode,
        Mode::Confirm {
            choice: confirmation::Choice::Return,
            ..
        }
    ));
    key(&mut s, KeyCode::Left);
    assert!(matches!(
        s.mode,
        Mode::Confirm {
            choice: confirmation::Choice::Execute,
            ..
        }
    ));
    key(&mut s, KeyCode::Esc);
    key(&mut s, KeyCode::Left);
    assert_eq!(s.page, Page::Repositories);
    assert_eq!(
        std::fs::read(s.service.store.config_path()).unwrap(),
        before
    );
}

#[test]
fn view_commands_open_full_views_and_return_to_the_focused_pane() {
    use crate::{
        navigation::View,
        workspace::{Arrangement, Section},
    };
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let f = support::Fixture::create();
        support::write(f.repo.join("a.ts"), "export const a = 1;\n");
        support::write(f.repo.join("b.ts"), "export const b = 2;\n");
        let (_home, mut s) = fixture(vec![f.repo.clone()]);
        key(&mut s, KeyCode::Enter);
        wait_workspace(&mut s);
        s.switch_repository_section(Section::Worktree);
        key(&mut s, KeyCode::Down);
        key(&mut s, KeyCode::Char(' '));
        let original = s.mode.file_session().unwrap().clone();
        for (name, view, prefix) in [
            ("repo diff", View::Diff, "/repo diff "),
            ("repo log", View::History, "/repo log "),
            ("repo branch", View::Branches, "/repo branch "),
            ("repo stash", View::Stashes, "/repo stash "),
        ] {
            key(&mut s, KeyCode::Char('/'));
            assert_eq!(s.bar.text(), "/repo diff ");
            render(&mut s, 120, 32);
            assert_eq!(s.workspace_areas.len(), 4);
            key(&mut s, KeyCode::Esc);
            command(&mut s, name);
            wait(&mut s);
            assert_eq!(s.page, Page::View(view));
            for (width, height) in [(60, 18), (120, 32)] {
                render(&mut s, width, height);
                assert!(s.workspace_areas.is_empty());
            }
            key(&mut s, KeyCode::Char('/'));
            assert_eq!(s.bar.text(), prefix);
            key(&mut s, KeyCode::Esc);
            let depth = s.history.len();
            command(&mut s, name);
            wait(&mut s);
            assert_eq!(s.history.len(), depth);
            key(&mut s, KeyCode::Esc);
            assert_eq!(s.page, Page::Detail);
            assert_eq!(Section::of(&s.mode), Section::Worktree);
            let restored = s.mode.file_session().unwrap();
            assert_eq!(restored.cursor, original.cursor);
            assert_eq!(restored.selected, original.selected);
            assert_eq!(
                s.workspace.session().unwrap().arrangement,
                Arrangement::Tiled
            );
        }
        s.shutdown();
    }
}

#[test]
fn application_pages_restore_the_full_view_and_pane_search() {
    let f = support::Fixture::create();
    support::write(f.repo.join("source.ts"), "export const value = 1;\n");
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo diff");
    wait(&mut s);
    s.paste("source");
    for name in ["settings", "schedule", "repos manage", "help"] {
        s.paste(&format!("//{name}"));
        key(&mut s, KeyCode::Enter);
        assert!(if name == "repos manage" {
            matches!(s.mode, Mode::Management(_))
        } else {
            matches!(s.mode, Mode::Browsing)
        });
        key(&mut s, KeyCode::Esc);
        if !matches!(s.mode, Mode::Changes(_)) {
            key(&mut s, KeyCode::Esc);
        }
        assert_eq!(s.page, Page::View(crate::navigation::View::Diff));
        assert_eq!(s.mode.file_session().unwrap().query, "source");
        assert_eq!(s.bar.text(), "source");
    }
    s.shutdown();
}

#[test]
fn staging_in_full_diff_returns_updated_files_to_the_original_pane() {
    let f = support::Fixture::create();
    support::write(f.repo.join("stage.ts"), "export const value = 1;\n");
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    key(&mut s, KeyCode::Enter);
    wait_workspace(&mut s);
    s.switch_repository_section(crate::workspace::Section::Worktree);
    command(&mut s, "repo diff");
    wait(&mut s);
    command(&mut s, "repo diff stage");
    wait(&mut s);
    key(&mut s, KeyCode::Esc);
    assert_eq!(s.page, Page::Detail);
    let files = s.mode.file_session().unwrap();
    assert!(files.rows().iter().any(|row| row.area == Area::Staged));
    s.shutdown();
}

#[test]
fn background_index_updates_reach_a_saved_view_without_replacing_settings() {
    let f = support::Fixture::create();
    support::write(f.repo.join("late.ts"), "export const value = 1;\n");
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo diff");
    wait(&mut s);
    command(&mut s, "repo diff stage");
    command(&mut s, "settings");
    wait(&mut s);
    assert_eq!(s.page, Page::Configuration(Configuration::General));
    assert!(matches!(s.mode, Mode::Browsing));
    key(&mut s, KeyCode::Esc);
    assert_eq!(s.page, Page::View(crate::navigation::View::Diff));
    assert!(s.mode.file_session().unwrap().rows().is_empty());
    command(&mut s, "repo diff staged");
    wait(&mut s);
    let files = s.mode.file_session().unwrap();
    assert!(files.rows().iter().any(|row| row.area == Area::Staged));
    assert!(s.repository_tasks.is_empty());
    s.shutdown();
}

#[test]
fn returning_to_a_loading_pane_uses_the_completed_snapshot() {
    let f = support::Fixture::create();
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    key(&mut s, KeyCode::Enter);
    s.switch_repository_section(crate::workspace::Section::Branches);
    assert!(matches!(s.mode, Mode::Panel(_)));
    command(&mut s, "settings");
    wait_workspace(&mut s);
    key(&mut s, KeyCode::Esc);
    assert_eq!(s.page, Page::Detail);
    assert!(matches!(s.mode, Mode::Branches(_)));
    s.shutdown();
}
