use super::*;
use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};

fn click(s: &mut Screen, x: u16, y: u16) {
    for kind in [
        MouseEventKind::Down(MouseButton::Left),
        MouseEventKind::Up(MouseButton::Left),
    ] {
        s.mouse(
            MouseEvent {
                kind,
                column: x,
                row: y,
                modifiers: KeyModifiers::NONE,
            },
            &Cancel::default(),
        )
        .unwrap();
    }
}
fn item_position(s: &mut Screen, target: crate::pointer::Target, index: usize) -> (u16, u16) {
    if target == crate::pointer::Target::Settings {
        s.setting_selected = index;
        s.settings_focus = None;
    }
    render(s, 120, 32);
    for y in 0..32 {
        for x in 0..120 {
            if s.pointer.hit(ratatui::layout::Position::new(x, y))
                == Some(crate::pointer::Hit::Item(target, index))
            {
                return (x, y);
            }
        }
    }
    panic!("missing rendered item {target:?}/{index}")
}

#[test]
fn double_click_opens_repository_once_and_single_click_only_selects() {
    let f = support::Fixture::create();
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    let (x, y) = item_position(&mut s, crate::pointer::Target::Repositories, 0);
    click(&mut s, x, y);
    assert_eq!(s.page, Page::Repositories);
    render(&mut s, 120, 32);
    click(&mut s, x, y);
    assert_eq!(s.page, Page::Detail);
    let depth = s.history.len();
    wait_workspace(&mut s);
    render(&mut s, 120, 32);
    click(&mut s, x, y);
    assert_eq!(s.history.len(), depth);
    assert!(matches!(s.clicks, crate::input::clicks::Clicks::Idle));
    s.shutdown();
}

#[test]
fn blank_scrollbar_keyboard_and_disabled_clicks_do_not_activate_rows() {
    let paths = (0..30)
        .map(|i| PathBuf::from(format!("/example/repo-{i:02}")))
        .collect();
    let (_home, mut s) = fixture(paths);
    let (x, y) = item_position(&mut s, crate::pointer::Target::Repositories, 0);
    click(&mut s, x, y);
    let area = s.repository_area.unwrap();
    click(&mut s, area.x, area.y);
    click(&mut s, x, y);
    assert_eq!(s.page, Page::Repositories);
    key(&mut s, KeyCode::Home);
    click(&mut s, x, y);
    assert_eq!(s.page, Page::Repositories);
    for _ in 0..2 {
        click(&mut s, area.right() - 1, area.bottom() - 2);
    }
    assert_eq!(s.page, Page::Repositories);
    assert_eq!(s.selected(), 29);
    assert!(matches!(s.clicks, crate::input::clicks::Clicks::Idle));
    s.config.mouse.precise_click = false;
    for _ in 0..2 {
        click(&mut s, x, y);
    }
    assert_eq!(s.page, Page::Repositories);
    assert_eq!(s.selected(), 29);
}

#[test]
fn double_click_uses_the_existing_file_branch_history_and_stash_enter_handlers() {
    let f = support::Fixture::create();
    f.commit("file.rs", "fn before() {}\n");
    support::write(f.repo.join("file.rs"), "fn after() {}\n");
    support::git(&f.repo, &["stash", "push", "-m", "double-click fixture"]);
    support::write(f.repo.join("file.rs"), "fn current() {}\n");
    let index = std::fs::read(f.repo.join(".git/index")).unwrap();
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    for (command_text, target) in [
        ("repo diff", crate::pointer::Target::Files),
        ("repo branch", crate::pointer::Target::Branches),
        ("repo log", crate::pointer::Target::History),
        ("repo stash", crate::pointer::Target::Stashes),
    ] {
        command(&mut s, command_text);
        wait(&mut s);
        let (x, y) = item_position(&mut s, target, 0);
        click(&mut s, x, y);
        render(&mut s, 120, 32);
        click(&mut s, x, y);
        wait(&mut s);
        match target {
            crate::pointer::Target::Files => assert!(
                matches!(&s.mode,Mode::Changes(session) if session.focus==crate::changes::Focus::Preview)
            ),
            crate::pointer::Target::Branches => assert!(
                matches!(&s.mode,Mode::Branches(view) if matches!(view.as_ref(),crate::branches::View::Detail{..}))
            ),
            crate::pointer::Target::History => assert!(
                matches!(&s.mode,Mode::History(view) if matches!(view.as_ref(),crate::history::View::Commit(_)))
            ),
            crate::pointer::Target::Stashes => assert!(
                matches!(&s.mode,Mode::Stashes(view) if matches!(view.as_ref(),crate::stashes::View::Detail{..}))
            ),
            _ => unreachable!(),
        }
    }
    assert_eq!(std::fs::read(f.repo.join(".git/index")).unwrap(), index);
    assert_eq!(
        std::fs::read_to_string(f.repo.join("file.rs")).unwrap(),
        "fn current() {}\n"
    );
    assert_eq!(support::git(&f.repo, &["stash", "list"]).lines().count(), 1);
    s.shutdown();
}

#[cfg(feature = "developer-scheduler")]
#[test]
fn settings_double_click_toggles_once_and_schedule_double_click_stops_at_confirmation() {
    let (_home, mut s) = fixture(vec![]);
    command(&mut s, "settings");
    let index = Configuration::General
        .items(&s.snapshot, &s.config.ai)
        .iter()
        .position(|item| *item == crate::settings::Item::Mouse(engine::state::MouseSetting::Scroll))
        .unwrap();
    let (x, y) = item_position(&mut s, crate::pointer::Target::Settings, index);
    click(&mut s, x, y);
    assert!(s.config.mouse.scroll);
    click(&mut s, x, y);
    wait(&mut s);
    assert!(!s.config.mouse.scroll);
    render(&mut s, 120, 32);
    click(&mut s, x, y);
    wait(&mut s);
    assert!(!s.config.mouse.scroll);
    command(&mut s, "schedule");
    let index = Configuration::Schedule
        .actions(&s.snapshot)
        .iter()
        .position(|action| *action == Action::DisableSchedule)
        .unwrap();
    let (x, y) = item_position(&mut s, crate::pointer::Target::Settings, index);
    click(&mut s, x, y);
    click(&mut s, x, y);
    assert!(matches!(
        s.mode,
        Mode::Confirm {
            operation: Confirmation::Schedule(_),
            ..
        }
    ));
    render(&mut s, 120, 32);
    click(&mut s, x, y);
    assert!(matches!(
        s.mode,
        Mode::Confirm {
            operation: Confirmation::Schedule(_),
            ..
        }
    ));
}
