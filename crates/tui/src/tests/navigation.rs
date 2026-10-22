use super::*;

#[test]
fn empty_input_arrows_open_details_and_return_without_losing_selection_or_history() {
    let (_home, mut s) = fixture(vec!["/example/alpha".into(), "/example/beta".into()]);
    key(&mut s, KeyCode::Down);
    key(&mut s, KeyCode::Right);
    assert_eq!(s.page, Page::Detail);
    assert_eq!(s.selected(), 1);
    assert_eq!(s.history, vec![Page::Repositories]);
    key(&mut s, KeyCode::F(1));
    assert_eq!(s.page, Page::Help);
    key(&mut s, KeyCode::Left);
    assert_eq!(s.page, Page::Detail);
    key(&mut s, KeyCode::Left);
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
    command(&mut s, "stage");
    wait(&mut s);
    assert!(matches!(s.mode, Mode::Files(_)));
    key(&mut s, KeyCode::Char(' '));
    key(&mut s, KeyCode::Right);
    wait(&mut s);
    assert!(matches!(&s.mode, Mode::Diff { .. }));
    assert!(render(&mut s, 100, 28).contains("navigation content"));
    key(&mut s, KeyCode::Left);
    assert!(
        matches!(&s.mode, Mode::Files(session) if session.cursor == 0 && session.selected.len() == 1)
    );
    key(&mut s, KeyCode::Left);
    assert!(matches!(s.mode, Mode::Browsing));
    assert_eq!(s.page, Page::Detail);
    key(&mut s, KeyCode::Left);
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
    s.paste("/repo add ~/中文");
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
    assert_eq!(s.bar.editor().text, "/settings concurrency 4");
    assert!(matches!(s.mode, Mode::Browsing));
    key(&mut s, KeyCode::Esc);
    key(&mut s, KeyCode::Left);
    assert_eq!(s.page, Page::Repositories);
    command(&mut s, "schedule");
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
