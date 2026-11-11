use super::*;
use crate::command::options::Value;
use crate::stash_save::{Focus, Scope};

fn mark(screen: &mut Screen, name: &str) {
    let Mode::Changes(session) = &mut screen.mode else {
        panic!("workspace changes")
    };
    let index = session
        .files
        .snapshot
        .changes
        .iter()
        .position(|change| change.location.name() == std::path::Path::new(name))
        .unwrap();
    session.files.mark_file(index);
}
fn review(screen: &Screen) -> &crate::stash_save::Session {
    let Mode::StashSave(session) = &screen.mode else {
        panic!("{:?}", screen.mode)
    };
    session
}
#[test]
fn stash_scope_menu_disables_unmarked_selection_and_completion_keeps_captured_checks() {
    for language in [i18n::Language::Chinese, i18n::Language::English] {
        let _language = i18n::using(language);
        let f = support::Fixture::create();
        f.commit("selected 中文.txt", "base\n");
        support::write(f.repo.join("selected 中文.txt"), "changed\n");
        support::write(f.repo.join("other.txt"), "untracked\n");
        let (_home, mut s) = fixture(vec![f.repo.clone()]);
        command(&mut s, "repo diff");
        wait(&mut s);
        let index = std::fs::read(f.repo.join(".git/index")).unwrap();
        command(&mut s, "repo stash save");
        assert_eq!(super::options::choices(&s).options.len(), 3);
        assert_eq!(super::options::choices(&s).current, None);
        assert!(render(&mut s, 120, 30).contains(i18n::tr!(StashSelectFiles)));
        key(&mut s, KeyCode::Down);
        key(&mut s, KeyCode::Down);
        key(&mut s, KeyCode::Enter);
        assert_eq!(s.message, i18n::tr!(StashSelectFiles));
        assert!(matches!(s.mode, Mode::Changes(_)));
        key(&mut s, KeyCode::Esc);
        key(&mut s, KeyCode::Esc);
        mark(&mut s, "selected 中文.txt");
        command(&mut s, "repo stash save");
        let text = render(&mut s, 120, 30);
        assert!(
            text.contains(&i18n::tf!(CommandTargetFiles, count = 1)),
            "{text}"
        );
        key(&mut s, KeyCode::Down);
        key(&mut s, KeyCode::Down);
        key(&mut s, KeyCode::Tab);
        assert!(s.bar.text().ends_with("save selected"));
        assert!(matches!(s.mode, Mode::Changes(_)));
        // 打开菜单后目标已冻结，后续刷新或变化不能把保存对象换成另一行。
        if let Mode::Changes(session) = &mut s.mode {
            session.files.selected.clear();
        }
        assert_eq!(
            super::options::choices(&s).options[2].value,
            Value::StashSave(Scope::Selected)
        );
        key(&mut s, KeyCode::Enter);
        wait(&mut s);
        assert_eq!(review(&s).plan.files, [PathBuf::from("selected 中文.txt")]);
        for (width, height) in [(60, 18), (120, 30), (24, 8), (1, 1), (0, 0)] {
            render(&mut s, width, height);
        }
        assert_eq!(std::fs::read(f.repo.join(".git/index")).unwrap(), index);
        assert!(
            s.service
                .stashes(&f.repo, &Cancel::default())
                .unwrap()
                .entries
                .is_empty()
        );
        key(&mut s, KeyCode::Esc);
        assert!(matches!(s.mode, Mode::Changes(_)));
        assert!(s.bar.text().ends_with("save selected"));
        assert_eq!(
            std::fs::read_to_string(f.repo.join("selected 中文.txt")).unwrap(),
            "changed\n"
        );
        s.shutdown();
    }
}

#[test]
fn stash_review_edits_optional_message_on_one_page_and_saves_only_checked_files() {
    let f = support::Fixture::create();
    f.commit("chosen.txt", "base\n");
    f.commit("other.txt", "other base\n");
    support::write(f.repo.join("chosen.txt"), "chosen change\n");
    support::write(f.repo.join("other.txt"), "unselected staged\n");
    support::git(&f.repo, &["add", "--", "other.txt"]);
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo diff");
    wait(&mut s);
    mark(&mut s, "chosen.txt");
    command(&mut s, "repo stash save selected");
    wait(&mut s);
    let initial = render(&mut s, 120, 30);
    assert!(
        initial.contains("chosen.txt") && !initial.contains("other.txt"),
        "{initial}"
    );
    let body = s.body_area;
    let input = s.input_area;
    s.paste("临时保存 中文");
    key(&mut s, KeyCode::Enter);
    assert_eq!(
        review(&s).focus,
        Focus::Actions(confirmation::Choice::Execute)
    );
    render(&mut s, 120, 30);
    assert_eq!(s.body_area, body);
    assert_eq!(s.input_area, input);
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    assert_eq!(
        std::fs::read_to_string(f.repo.join("chosen.txt")).unwrap(),
        "base\n"
    );
    assert_eq!(
        std::fs::read_to_string(f.repo.join("other.txt")).unwrap(),
        "unselected staged\n"
    );
    assert_eq!(
        support::git(&f.repo, &["show", ":other.txt"]),
        "unselected staged"
    );
    assert!(
        s.service
            .stashes(&f.repo, &Cancel::default())
            .unwrap()
            .entries[0]
            .message
            .contains("临时保存 中文")
    );
    assert!(!support::git(&f.repo, &["diff", "stash^1", "stash", "--name-only"]).contains("other"));
    s.shutdown();
}

#[test]
fn all_and_tracked_ignore_checks_and_blank_message_uses_git_default() {
    for (scope, contains_new) in [("tracked", false), ("all", true)] {
        let f = support::Fixture::create();
        f.commit("first", "base\n");
        f.commit("second", "base\n");
        support::write(f.repo.join("first"), "first change\n");
        support::write(f.repo.join("second"), "second change\n");
        support::write(f.repo.join("new file"), "new\n");
        let (_home, mut s) = fixture(vec![f.repo.clone()]);
        command(&mut s, "repo diff");
        wait(&mut s);
        mark(&mut s, "first");
        command(&mut s, &format!("repo stash save {scope}"));
        wait(&mut s);
        assert!(review(&s).plan.files.contains(&PathBuf::from("second")));
        assert!(review(&s).message.text.is_empty());
        key(&mut s, KeyCode::BackTab);
        key(&mut s, KeyCode::Enter);
        wait(&mut s);
        assert_eq!(
            std::fs::read_to_string(f.repo.join("second")).unwrap(),
            "base\n"
        );
        assert_eq!(!f.repo.join("new file").exists(), contains_new);
        assert!(
            s.service
                .stashes(&f.repo, &Cancel::default())
                .unwrap()
                .entries[0]
                .message
                .contains("WIP on main:")
        );
        s.shutdown();
    }
}

#[test]
fn stash_review_pointer_and_tab_choose_actions_and_double_click_matches_enter() {
    use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
    let f = support::Fixture::create();
    f.commit("file", "base\n");
    support::write(f.repo.join("file"), "changed\n");
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo stash save tracked");
    wait(&mut s);
    render(&mut s, 100, 28);
    let area = crate::view::input::content_area(s.input_area.unwrap());
    let execute = crate::view::stash_save::actions(area)[0].0;
    let click = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: execute.x,
        row: execute.y,
        modifiers: KeyModifiers::NONE,
    };
    s.config.mouse.precise_click = false;
    assert!(!s.mouse(click, &Cancel::default()).unwrap());
    assert_eq!(review(&s).focus, Focus::Message);
    s.config.mouse.precise_click = true;
    assert!(s.mouse(click, &Cancel::default()).unwrap());
    assert_eq!(
        review(&s).focus,
        Focus::Actions(confirmation::Choice::Execute)
    );
    assert!(
        s.service
            .stashes(&f.repo, &Cancel::default())
            .unwrap()
            .entries
            .is_empty()
    );
    assert!(s.mouse(click, &Cancel::default()).unwrap());
    wait(&mut s);
    assert_eq!(
        std::fs::read_to_string(f.repo.join("file")).unwrap(),
        "base\n"
    );
    s.shutdown();
}

#[test]
fn long_stash_reviews_support_wheel_and_scrollbar_drag_without_moving_the_message_field() {
    use crate::pointer::{Action, Hit, ScrollTarget};
    use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
    use ratatui::layout::Position;
    let f = support::Fixture::create();
    for n in 0..50 {
        support::write(f.repo.join(format!("file-{n:02}.txt")), "new\n");
    }
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo stash save all");
    wait(&mut s);
    render(&mut s, 100, 28);
    let body = s.body_area.unwrap();
    let input = s.input_area;
    let point = Position::new(body.x + 3, body.y + 3);
    let wheel = MouseEvent {
        kind: MouseEventKind::ScrollDown,
        column: point.x,
        row: point.y,
        modifiers: KeyModifiers::NONE,
    };
    s.config.mouse.scroll = false;
    assert!(!s.mouse(wheel, &Cancel::default()).unwrap());
    assert_eq!(review(&s).scroll, 0);
    s.config.mouse.scroll = true;
    assert!(s.mouse(wheel, &Cancel::default()).unwrap());
    render(&mut s, 100, 28);
    assert_eq!(review(&s).scroll, 3);
    let message = crate::view::stash_save::message_area(body);
    let thumb = (body.y..message.y)
        .filter_map(|y| {
            let point = Position::new(body.right() - 1, y);
            matches!(
                s.pointer.hit(point),
                Some(Hit::Scrollbar(Action::Scroll(ScrollTarget::Text, _)))
            )
            .then_some(point)
            .filter(|point| s.pointer.grab(*point).is_some())
        })
        .next()
        .unwrap();
    let press = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: thumb.x,
        row: thumb.y,
        modifiers: KeyModifiers::NONE,
    };
    assert!(s.mouse(press, &Cancel::default()).unwrap());
    let drag = MouseEvent {
        kind: MouseEventKind::Drag(MouseButton::Left),
        row: message.y - 2,
        ..press
    };
    assert!(s.mouse(drag, &Cancel::default()).unwrap());
    render(&mut s, 100, 28);
    assert!(review(&s).scroll > 3);
    assert_eq!(s.input_area, input);
    assert_eq!(
        crate::view::stash_save::message_area(s.body_area.unwrap()),
        message
    );
    key(&mut s, KeyCode::Esc);
    assert!(
        s.service
            .stashes(&f.repo, &Cancel::default())
            .unwrap()
            .entries
            .is_empty()
    );
    s.shutdown();
}

#[test]
fn stash_save_shift_tab_changes_mode_without_losing_text_and_tab_never_saves() {
    let f = support::Fixture::create();
    f.commit("draft.txt", "base\n");
    support::write(f.repo.join("draft.txt"), "changed\n");
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo stash save tracked");
    wait(&mut s);
    s.paste("draft 中文");
    let before = (s.body_area, s.input_area);
    key(&mut s, KeyCode::Tab);
    assert_eq!(review(&s).focus, crate::stash_save::Focus::Message);
    key(&mut s, KeyCode::BackTab);
    assert!(matches!(
        review(&s).focus,
        crate::stash_save::Focus::Actions(_)
    ));
    key(&mut s, KeyCode::Tab);
    assert!(matches!(
        review(&s).focus,
        crate::stash_save::Focus::Actions(_)
    ));
    key(&mut s, KeyCode::Esc);
    assert_eq!(review(&s).message.text, "draft 中文");
    assert_eq!(review(&s).focus, crate::stash_save::Focus::Message);
    key(&mut s, KeyCode::BackTab);
    s.paste(" 继续编辑");
    assert_eq!(review(&s).message.text, "draft 中文 继续编辑");
    assert_eq!(review(&s).focus, crate::stash_save::Focus::Message);
    assert_eq!((s.body_area, s.input_area), before);
    assert_eq!(
        std::fs::read_to_string(f.repo.join("draft.txt")).unwrap(),
        "changed\n"
    );
    s.shutdown();
}
