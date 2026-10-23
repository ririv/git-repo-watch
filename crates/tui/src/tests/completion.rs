use super::*;

#[test]
fn directory_completion_accepts_a_candidate_before_review_and_preserves_its_origin() {
    let temp = tempfile::tempdir().unwrap();
    for name in ["仓库 甲", "仓库 乙"] {
        std::fs::create_dir(temp.path().join(name)).unwrap();
    }
    let (_home, mut s) = fixture(vec![]);
    command(&mut s, "repos manage");
    s.paste(&format!("//repos add \"{}/仓\"", temp.path().display()));
    key(&mut s, KeyCode::Tab);
    assert!(matches!(
        &s.bar,
        bar::InputBar::Command {
            menu: command::path::Menu::Directories(_),
            ..
        }
    ));
    assert!(s.bar.text().ends_with("/仓库 \""));
    assert_eq!(s.bar.editor().cursor, s.bar.editor().text.len() - 1);
    assert_eq!(s.page, Page::Configuration(Configuration::Repositories));
    assert!(matches!(s.mode, Mode::Browsing));
    key(&mut s, KeyCode::Down);
    key(&mut s, KeyCode::BackTab);
    key(&mut s, KeyCode::Tab);
    key(&mut s, KeyCode::Enter);
    assert!(matches!(
        &s.bar,
        bar::InputBar::Command {
            menu: command::path::Menu::Commands,
            ..
        }
    ));
    assert!(s.bar.text().ends_with("/仓库 甲/\""));
    assert!(matches!(s.mode, Mode::Browsing));
    assert!(s.service.store.config().unwrap().repositories.is_empty());
    key(&mut s, KeyCode::Enter);
    assert!(matches!(&s.mode, Mode::Confirm {
        operation: Confirmation::Configure { value, .. }, ..
    } if value == &format!("{}/仓库 甲/", temp.path().display())));
    key(&mut s, KeyCode::Esc);
    assert!(s.bar.text().ends_with("/仓库 甲/\""));
    assert_eq!(s.page, Page::Configuration(Configuration::Repositories));
    assert!(s.service.store.config().unwrap().repositories.is_empty());
}

#[test]
fn completion_menu_closes_on_escape_edit_or_paste_and_errors_keep_the_input() {
    let temp = tempfile::tempdir().unwrap();
    for name in ["one", "other"] {
        std::fs::create_dir(temp.path().join(name)).unwrap();
    }
    let (_home, mut s) = fixture(vec![]);
    for input in [
        "repos add",
        "repos exclude",
        "repos unexclude",
        "repos scan",
        "repos import",
    ] {
        s.paste(&format!("/{input} {}/o", temp.path().display()));
        key(&mut s, KeyCode::Tab);
        let draft = s.bar.editor().text.clone();
        key(&mut s, KeyCode::Esc);
        assert_eq!(s.bar.editor().text, draft);
        assert!(matches!(s.mode, Mode::Browsing));
        key(&mut s, KeyCode::Tab);
        key(&mut s, KeyCode::Char('n'));
        assert!(matches!(
            &s.bar,
            bar::InputBar::Command {
                menu: command::path::Menu::Commands,
                ..
            }
        ));
        key(&mut s, KeyCode::Tab);
        assert!(s.bar.text().ends_with("/one/"));
        key(&mut s, KeyCode::Esc);
    }
    s.paste(&format!("//repos add {}/o", temp.path().display()));
    key(&mut s, KeyCode::Tab);
    s.paste("ne");
    assert!(matches!(
        &s.bar,
        bar::InputBar::Command {
            menu: command::path::Menu::Commands,
            ..
        }
    ));
    key(&mut s, KeyCode::Tab);
    assert!(s.bar.text().ends_with("/one/"));
    s.paste("missing/");
    let draft = s.bar.editor().text.clone();
    key(&mut s, KeyCode::Tab);
    assert!(!s.message.is_empty());
    assert_eq!(s.bar.editor().text, draft);
    assert!(matches!(s.mode, Mode::Browsing));
    assert!(s.service.store.config().unwrap().repositories.is_empty());
}

#[test]
fn directory_candidates_share_the_full_width_panel_in_both_languages_and_scroll_to_selection() {
    let temp = tempfile::tempdir().unwrap();
    for index in 0..12 {
        std::fs::create_dir(temp.path().join(format!("目录 {index:02}"))).unwrap();
    }
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let (_home, mut s) = fixture(vec![]);
        s.paste(&format!("//repos add {}/目", temp.path().display()));
        key(&mut s, KeyCode::Tab);
        for (width, height) in [(60, 18), (100, 28), (150, 32)] {
            let rendered = render(&mut s, width, height);
            assert!(
                rendered.contains(i18n::tr!(DirectoryCandidates)),
                "{rendered}"
            );
            assert!(rendered.contains("目录 00/"));
            assert!(!rendered.contains("目录 08/"));
            assert_eq!(rendered.lines().count(), height as usize);
        }
        key(&mut s, KeyCode::BackTab);
        let rendered = render(&mut s, 100, 28);
        assert!(rendered.contains("目录 11/"));
        assert!(rendered.contains(i18n::tr!(DirectoryCompletionHint)));
        assert!(!rendered.contains("目录 00/"));
        key(&mut s, KeyCode::Enter);
        assert!(s.bar.text().ends_with("/目录 11/"));
    }
}

#[test]
fn all_path_commands_expand_tilde_in_review_and_return_to_the_original_draft() {
    let (home, mut s) = fixture(vec![]);
    for name in ["add", "exclude", "unexclude", "scan", "import"] {
        for quote in ["", "'", "\""] {
            let draft = format!("repos {name} {quote}~/中文 空格/{quote}");
            command(&mut s, &draft);
            assert!(matches!(&s.mode, Mode::Confirm {
                operation: Confirmation::Configure { value, .. }, ..
            } if std::path::Path::new(value) == home.path().join("中文 空格")));
            key(&mut s, KeyCode::Esc);
            assert_eq!(s.bar.text(), format!("/{draft}"));
            assert!(s.service.store.config().unwrap().repositories.is_empty());
        }
    }
    command(&mut s, "repos exclude ~");
    assert!(matches!(&s.mode, Mode::Confirm {
        operation: Confirmation::Configure { value, .. }, ..
    } if std::path::Path::new(value) == home.path()));
}

#[test]
fn tilde_completed_repository_add_executes_with_the_same_home_and_saves_an_absolute_path() {
    let (home, mut s) = fixture(vec![]);
    let repository = home.path().join("仓库 空格");
    support::git(home.path(), &["init", "仓库 空格"]);
    s.paste("//repos add \"~/仓\"");
    key(&mut s, KeyCode::Tab);
    assert_eq!(s.bar.text(), "/repos add \"~/仓库 空格/\"");
    assert!(matches!(s.mode, Mode::Browsing));
    key(&mut s, KeyCode::Enter);
    assert!(matches!(&s.mode, Mode::Confirm {
        operation: Confirmation::Configure { value, .. }, ..
    } if std::path::Path::new(value) == repository));
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    assert!(matches!(s.mode, Mode::Browsing));
    assert_eq!(
        s.service.store.config().unwrap().repositories,
        vec![std::fs::canonicalize(repository).unwrap()]
    );
    assert!(s.service.store.report().unwrap().is_none());
}
