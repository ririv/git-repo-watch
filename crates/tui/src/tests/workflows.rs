use super::*;
use crate::{
    pointer::{Hit, Target},
    workflow::{Action as Edit, Cursor, FooterAction, RowAction, Session},
};
use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use engine::workflow::{Plan, Step, Trigger};
use ratatui::layout::Position;

fn click_item(screen: &mut Screen, target: Target, index: usize) {
    render(screen, 120, 32);
    let position = (0..32)
        .flat_map(|y| (0..120).map(move |x| Position::new(x, y)))
        .find(|position| screen.pointer.hit(*position) == Some(Hit::Item(target, index)))
        .unwrap();
    screen
        .mouse(
            MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: position.x,
                row: position.y,
                modifiers: KeyModifiers::NONE,
            },
            &Cancel::default(),
        )
        .unwrap();
}
fn click_action(screen: &mut Screen, action: Edit) {
    let index = Edit::ALL.iter().position(|item| *item == action).unwrap();
    click_item(screen, Target::WorkflowActions, index);
}
fn session(screen: &Screen) -> &Session {
    let Mode::Workflow(session) = &screen.mode else {
        panic!("expected workflow editor")
    };
    session
}

#[test]
fn keyboard_checkboxes_and_save_are_reachable_without_switching_focus_modes() {
    let (_home, mut s) = fixture(vec![]);
    let scheduled = s.config.scheduled_actions.clone();
    command(&mut s, "settings startup");
    assert_eq!(s.page, Page::Configuration(Configuration::General));
    key(&mut s, KeyCode::Down);
    key(&mut s, KeyCode::Enter);
    key(&mut s, KeyCode::Down);
    key(&mut s, KeyCode::Char(' '));
    assert_eq!(
        session(&s).steps(),
        vec![Step::Fetch, Step::Pull, Step::Push, Step::Inspect]
    );
    assert_eq!(
        s.service.store.config().unwrap().startup_actions,
        Plan::fetch()
    );
    key(&mut s, KeyCode::Down);
    key(&mut s, KeyCode::Down);
    assert!(matches!(
        session(&s).cursor,
        Cursor::Footer {
            action: FooterAction::Save,
            ..
        }
    ));
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    assert_eq!(
        s.config.startup_actions,
        "fetch pull push inspect".parse().unwrap()
    );
    assert_eq!(s.config.scheduled_actions, scheduled);
    assert!(s.service.store.report().unwrap().is_none());
    assert!(s.check_task.is_none());
    command(&mut s, "schedule actions");
    assert_eq!(s.page, Page::Configuration(Configuration::Schedule));
    click_item(&mut s, Target::WorkflowToggle, 0);
    click_action(&mut s, Edit::Save);
    wait(&mut s);
    assert_eq!(s.config.scheduled_actions, Plan::local());
    assert_eq!(
        s.config.startup_actions,
        "fetch pull push inspect".parse().unwrap()
    );
}

#[test]
fn invalid_push_stays_visible_and_a_checkbox_can_fix_the_draft() {
    let (_home, mut s) = fixture(vec![]);
    command(&mut s, "settings startup");
    click_item(&mut s, Target::WorkflowToggle, 0);
    click_item(&mut s, Target::WorkflowToggle, 2);
    click_action(&mut s, Edit::Save);
    assert!(matches!(s.mode, Mode::Workflow(_)));
    assert!(s.message.contains(i18n::tr!(WorkflowPushNeedsRefresh)));
    assert!(render(&mut s, 120, 32).contains(i18n::tr!(WorkflowPushNeedsRefresh)));
    assert_eq!(
        s.service.store.config().unwrap().startup_actions,
        Plan::fetch()
    );
    click_item(&mut s, Target::WorkflowToggle, 0);
    assert_eq!(
        session(&s).plan().unwrap(),
        "fetch push inspect".parse().unwrap()
    );
    key(&mut s, KeyCode::Esc);
    assert!(matches!(s.mode, Mode::Browsing));
    assert_eq!(s.config.startup_actions, Plan::fetch());
    assert_eq!(
        s.service.store.config().unwrap().startup_actions,
        Plan::fetch()
    );
}

#[test]
fn row_controls_move_the_selected_operation_and_preserve_saved_order() {
    let (_home, mut s) = fixture(vec![]);
    command(&mut s, "settings startup");
    key(&mut s, KeyCode::Down);
    key(&mut s, KeyCode::Enter);
    key(&mut s, KeyCode::Right);
    assert!(matches!(
        session(&s).cursor,
        Cursor::Step {
            index: 1,
            action: RowAction::Up
        }
    ));
    key(&mut s, KeyCode::Enter);
    assert_eq!(session(&s).selected_step(), Step::Pull);
    assert_eq!(
        session(&s).steps(),
        vec![Step::Pull, Step::Fetch, Step::Inspect]
    );
    click_action(&mut s, Edit::Down);
    assert_eq!(session(&s).selected_step(), Step::Pull);
    assert_eq!(
        session(&s).steps(),
        vec![Step::Fetch, Step::Pull, Step::Inspect]
    );
    s.mode = Mode::Workflow(Box::new(Session::new(
        Trigger::Startup,
        &"pull fetch push inspect".parse().unwrap(),
    )));
    assert_eq!(
        session(&s).steps(),
        vec![Step::Pull, Step::Fetch, Step::Push, Step::Inspect]
    );
    click_action(&mut s, Edit::Save);
    wait(&mut s);
    command(&mut s, "settings startup");
    assert_eq!(
        session(&s).steps(),
        vec![Step::Pull, Step::Fetch, Step::Push, Step::Inspect]
    );
}

#[test]
fn all_available_steps_and_save_controls_are_visible_in_both_languages() {
    for language in [i18n::Language::Chinese, i18n::Language::English] {
        let _language = i18n::using(language);
        let (_home, mut s) = fixture(vec![]);
        command(&mut s, "schedule actions");
        for (width, height) in [(60, 18), (100, 28), (180, 36)] {
            let text = render(&mut s, width, height);
            let lines = text.lines().collect::<Vec<_>>();
            let actions_border = lines
                .iter()
                .position(|line| line.contains(i18n::tr!(WorkflowControls)))
                .unwrap();
            assert!(lines[actions_border].starts_with('┌'), "{text}");
            assert!(
                lines[actions_border - 1].trim_end().ends_with('┘'),
                "{text}"
            );
            for y in 0..height {
                for x in 0..width {
                    match s.pointer.hit(Position::new(x, y)) {
                        Some(Hit::Item(Target::WorkflowToggle | Target::WorkflowSteps, _)) => {
                            assert!(usize::from(y) < actions_border, "{text}");
                        }
                        Some(Hit::Item(Target::WorkflowActions, _)) => {
                            assert!(usize::from(y) > actions_border, "{text}");
                        }
                        _ => {}
                    }
                }
            }
            for step in [Step::Fetch, Step::Pull, Step::Push, Step::Inspect] {
                assert!(text.contains(step.token()), "{text}");
            }
            assert!(text.contains("[✓]"), "{text}");
            assert!(text.contains("[ ]"), "{text}");
            assert!(text.contains(i18n::tr!(WorkflowSave)), "{text}");
            assert!(text.contains(i18n::tr!(WorkflowCancel)), "{text}");
            assert!(
                text.contains(i18n::tr!(WorkflowEnableStep))
                    || text.contains(i18n::tr!(WorkflowDisableStep)),
                "{text}"
            );
        }
        let mut terminal = Terminal::new(TestBackend::new(120, 32)).unwrap();
        terminal.draw(|frame| view::render(frame, &mut s)).unwrap();
        assert!(
            terminal
                .backend()
                .buffer()
                .content
                .iter()
                .any(|cell| cell.fg == ratatui::style::Color::LightGreen
                    && cell.bg == ratatui::style::Color::Rgb(25, 65, 95))
        );
    }
}

#[test]
fn narrow_editor_reveals_footer_controls_and_precise_click_setting_is_respected() {
    let (_home, mut s) = fixture(vec![]);
    command(&mut s, "settings startup");
    for code in [
        KeyCode::End,
        KeyCode::Left,
        KeyCode::Home,
        KeyCode::Down,
        KeyCode::Right,
    ] {
        key(&mut s, code);
        render(&mut s, 28, 8);
    }
    key(&mut s, KeyCode::End);
    let text = render(&mut s, 28, 8);
    assert!(text.contains(i18n::tr!(WorkflowCancel)), "{text}");
    s.config.mouse.precise_click = false;
    click_item(&mut s, Target::WorkflowToggle, 1);
    assert_eq!(session(&s).steps(), Plan::fetch().steps());
    key(&mut s, KeyCode::Char(' '));
    assert!(matches!(s.mode, Mode::Browsing));
}

#[test]
fn editor_breadcrumb_has_one_settings_page_and_one_editor_level() {
    let (_home, mut s) = fixture(vec![]);
    for (command_name, section, title) in [
        (
            "settings startup",
            Configuration::General,
            i18n::tr!(WorkflowStartupActions),
        ),
        (
            "schedule actions",
            Configuration::Schedule,
            i18n::tr!(WorkflowScheduledActions),
        ),
    ] {
        command(&mut s, command_name);
        let labels = s
            .breadcrumbs()
            .into_iter()
            .map(|crumb| crumb.label)
            .collect::<Vec<_>>();
        assert_eq!(
            labels,
            vec![i18n::tr!(Repositories), section.title(), title]
        );
        key(&mut s, KeyCode::Esc);
        assert_eq!(s.page, Page::Configuration(section));
        key(&mut s, KeyCode::Esc);
    }
}

#[test]
fn selection_and_checkbox_changes_keep_workflow_regions_fixed() {
    fn anchors(screen: &Screen, width: u16, height: u16) -> Vec<Position> {
        let mut items = (0..3)
            .map(|index| Hit::Item(Target::WorkflowToggle, index))
            .collect::<Vec<_>>();
        items.push(Hit::Item(Target::WorkflowSteps, 3));
        for action in [Edit::Save, Edit::Cancel] {
            items.push(Hit::Item(
                Target::WorkflowActions,
                Edit::ALL.iter().position(|item| *item == action).unwrap(),
            ));
        }
        items
            .into_iter()
            .map(|hit| {
                (0..height)
                    .flat_map(|y| (0..width).map(move |x| Position::new(x, y)))
                    .find(|point| screen.pointer.hit(*point) == Some(hit))
                    .unwrap()
            })
            .collect()
    }
    for language in [i18n::Language::Chinese, i18n::Language::English] {
        let _language = i18n::using(language);
        for (width, height) in [(60, 18), (100, 28)] {
            let (_home, mut s) = fixture(vec![]);
            command(&mut s, "schedule actions");
            render(&mut s, width, height);
            let expected = anchors(&s, width, height);
            let description = session(&s).revealed.unwrap().1;
            // 焦点经过每个步骤和底部按钮，列表与说明区域始终保持位置。
            let navigation =
                std::iter::repeat_n(KeyCode::Down, 6).chain(std::iter::repeat_n(KeyCode::Up, 6));
            // 加入、移除步骤及显示校验错误同样不改变页面结构。
            let editing = [
                KeyCode::Home,
                KeyCode::Enter,
                KeyCode::Down,
                KeyCode::Down,
                KeyCode::Enter,
                KeyCode::Up,
                KeyCode::Enter,
            ];
            for code in navigation.chain(editing) {
                key(&mut s, code);
                render(&mut s, width, height);
                assert_eq!(anchors(&s, width, height), expected);
                assert_eq!(session(&s).revealed.unwrap().1, description);
            }
        }
    }
}
