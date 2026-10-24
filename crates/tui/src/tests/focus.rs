use super::*;
use crate::focus::State;

#[test]
fn global_focus_mode_preserves_search_selection_navigation_depth_and_git_state() {
    use crate::workspace::Section;
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let f = support::Fixture::create();
        f.commit("tracked", "before\n");
        support::write(f.repo.join("tracked"), "after\n");
        let index = std::fs::read(f.repo.join(".git/index")).unwrap();
        let (_home, mut s) = fixture(vec![f.repo.clone()]);
        key(&mut s, KeyCode::Enter);
        wait_workspace(&mut s);
        render(&mut s, 160, 36);
        command(&mut s, "focus");
        assert_eq!(s.focus_mode, State::Selecting);
        assert!(render(&mut s, 160, 36).contains(i18n::tr!(PaneFocusTitle)));
        for (code, section) in [
            (KeyCode::Right, Section::Branches),
            (KeyCode::Right, Section::Branches),
            (KeyCode::Down, Section::History),
            (KeyCode::Left, Section::Worktree),
            (KeyCode::Up, Section::Overview),
            (KeyCode::Up, Section::Overview),
        ] {
            key(&mut s, code);
            assert_eq!(Section::focused(&s.mode), Some(section));
            assert_eq!(s.breadcrumbs().len(), 3);
        }
        key(&mut s, KeyCode::Down);
        key(&mut s, KeyCode::Enter);
        assert_eq!(s.focus_mode, State::Inactive);
        s.paste("tracked");
        let depth = s.history.len();
        s.paste("//focus");
        key(&mut s, KeyCode::Enter);
        key(&mut s, KeyCode::Char('x'));
        s.paste("ignored paste");
        assert_eq!(s.bar.text(), "tracked");
        key(&mut s, KeyCode::Right);
        key(&mut s, KeyCode::Left);
        assert_eq!(s.bar.text(), "tracked");
        key(&mut s, KeyCode::Esc);
        assert_eq!(s.history.len(), depth);
        assert_eq!(s.focus_mode, State::Inactive);
        assert_eq!(s.bar.text(), "tracked");
        s.paste(" text");
        assert_eq!(s.bar.text(), "tracked text");
        for (code, modifiers) in [
            ('s', KeyModifiers::CONTROL),
            ('d', KeyModifiers::CONTROL),
            ('s', KeyModifiers::ALT),
            ('u', KeyModifiers::ALT),
        ] {
            s.key(
                KeyEvent::new(KeyCode::Char(code), modifiers),
                &Cancel::default(),
            )
            .unwrap();
            assert_eq!(Section::focused(&s.mode), Some(Section::Worktree));
        }
        assert_eq!(std::fs::read(f.repo.join(".git/index")).unwrap(), index);
        s.shutdown();
    }
}

#[test]
fn focus_mode_applies_to_diff_and_file_browser_splits_at_both_layouts() {
    use super::browser::{ready, select, session};
    use crate::{browser::opening::TextMode, changes::Focus};
    let f = support::Fixture::create();
    f.commit("README.md", "# Title\n\nbody\n");
    support::write(f.repo.join("README.md"), "# Title\n\nchanged\n");
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo diff");
    wait(&mut s);
    for (width, forward, back) in [
        (160, KeyCode::Right, KeyCode::Left),
        (80, KeyCode::Down, KeyCode::Up),
    ] {
        render(&mut s, width, 32);
        command(&mut s, "focus");
        key(&mut s, forward);
        assert!(matches!(&s.mode, Mode::Changes(view) if view.focus == Focus::Preview));
        key(&mut s, back);
        assert!(matches!(&s.mode, Mode::Changes(view) if view.focus == Focus::Files));
        key(&mut s, KeyCode::Esc);
    }
    command(&mut s, "repo files");
    ready(&mut s);
    select(&mut s, "README.md");
    command(&mut s, "repo files split");
    ready(&mut s);
    for width in [160, 24] {
        session(&mut s).focus = crate::browser::Focus::Tree;
        render(&mut s, width, 32);
        command(&mut s, "focus");
        key(&mut s, KeyCode::Right);
        assert_eq!(session(&mut s).presentation().active(), TextMode::Source);
        key(
            &mut s,
            if width == 160 {
                KeyCode::Right
            } else {
                KeyCode::Down
            },
        );
        assert_eq!(session(&mut s).presentation().active(), TextMode::Preview);
        key(&mut s, KeyCode::BackTab);
        assert_eq!(session(&mut s).presentation().active(), TextMode::Source);
        key(&mut s, KeyCode::Left);
        assert_eq!(session(&mut s).focus, crate::browser::Focus::Tree);
        key(&mut s, KeyCode::Enter);
    }
    s.shutdown();
}

#[test]
fn focus_mode_opens_global_commands_and_leaves_unrelated_pages_usable() {
    let f = support::Fixture::create();
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "focus");
    assert_eq!(s.focus_mode, State::Inactive);
    assert_eq!(s.message, i18n::tr!(PaneUnavailable));
    key(&mut s, KeyCode::Enter);
    wait_workspace(&mut s);
    command(&mut s, "focus");
    key(&mut s, KeyCode::Right);
    key(&mut s, KeyCode::Char('/'));
    assert_eq!(s.focus_mode, State::Inactive);
    assert_eq!(s.bar.text(), "/");
    s.paste("zoom");
    key(&mut s, KeyCode::Enter);
    assert_eq!(
        s.workspace.session().unwrap().arrangement,
        crate::workspace::Arrangement::Maximized
    );
    command(&mut s, "focus");
    key(&mut s, KeyCode::Char('/'));
    s.paste("settings");
    key(&mut s, KeyCode::Enter);
    assert_eq!(s.page, Page::Configuration(Configuration::General));
    assert_eq!(s.focus_mode, State::Inactive);
    s.shutdown();
}
