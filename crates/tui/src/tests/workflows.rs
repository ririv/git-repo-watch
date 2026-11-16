use super::*;
use crate::{
    pointer::{Hit, Target},
    workflow::{Action as Edit, Cursor, FooterAction, RowAction, Session},
};
use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use engine::workflow::{Plan, Step};
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
    let page = if matches!(action, Edit::Save | Edit::Cancel) {
        crate::input_pages::Pair::Actions
    } else {
        crate::input_pages::Pair::Content
    };
    if session(screen).page() != page {
        key(screen, KeyCode::BackTab);
    }
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
    command(&mut s, "schedule actions");
    assert_eq!(s.page, Page::Configuration(Configuration::Schedule));
    key(&mut s, KeyCode::Down);
    key(&mut s, KeyCode::Enter);
    key(&mut s, KeyCode::Down);
    key(&mut s, KeyCode::Char(' '));
    assert_eq!(
        session(&s).steps(),
        vec![Step::Fetch, Step::Pull, Step::Push, Step::Inspect]
    );
    assert_eq!(
        s.service.store.config().unwrap().scheduled_actions,
        Plan::fetch()
    );
    key(&mut s, KeyCode::Down);
    key(&mut s, KeyCode::BackTab);
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
        s.config.scheduled_actions,
        "fetch pull push inspect".parse().unwrap()
    );
    assert!(s.service.store.report().unwrap().is_none());
    assert!(s.check_task.is_none());
}

#[test]
fn invalid_push_stays_visible_and_a_checkbox_can_fix_the_draft() {
    let (_home, mut s) = fixture(vec![]);
    command(&mut s, "schedule actions");
    click_item(&mut s, Target::WorkflowToggle, 0);
    click_item(&mut s, Target::WorkflowToggle, 2);
    click_action(&mut s, Edit::Save);
    assert!(matches!(s.mode, Mode::Workflow(_)));
    assert!(s.message.contains(i18n::tr!(WorkflowPushNeedsRefresh)));
    assert!(render(&mut s, 120, 32).contains(i18n::tr!(WorkflowPushNeedsRefresh)));
    assert_eq!(
        s.service.store.config().unwrap().scheduled_actions,
        Plan::fetch()
    );
    click_item(&mut s, Target::WorkflowToggle, 0);
    assert_eq!(
        session(&s).plan().unwrap(),
        "fetch push inspect".parse().unwrap()
    );
    key(&mut s, KeyCode::Esc);
    assert!(matches!(s.mode, Mode::Browsing));
    assert_eq!(s.config.scheduled_actions, Plan::fetch());
    assert_eq!(
        s.service.store.config().unwrap().scheduled_actions,
        Plan::fetch()
    );
}

#[test]
fn row_controls_move_the_selected_operation_and_preserve_saved_order() {
    let (_home, mut s) = fixture(vec![]);
    command(&mut s, "schedule actions");
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
        &"pull fetch push inspect".parse().unwrap(),
    )));
    assert_eq!(
        session(&s).steps(),
        vec![Step::Pull, Step::Fetch, Step::Push, Step::Inspect]
    );
    click_action(&mut s, Edit::Save);
    wait(&mut s);
    command(&mut s, "schedule actions");
    assert_eq!(
        session(&s).steps(),
        vec![Step::Pull, Step::Fetch, Step::Push, Step::Inspect]
    );
}

#[test]
fn workflow_action_pages_share_the_footer_and_leave_body_geometry_unchanged() {
    for language in [i18n::Language::Chinese, i18n::Language::English] {
        let _language = i18n::using(language);
        let (_home, mut s) = fixture(vec![]);
        command(&mut s, "schedule actions");
        for (width, height) in [(60, 18), (100, 28), (180, 36)] {
            let text = render(&mut s, width, height);
            let body = s.body_area;
            let input = s.input_area.unwrap();
            let description = session(&s).revealed.unwrap().1;
            for step in [Step::Fetch, Step::Pull, Step::Push, Step::Inspect] {
                assert!(text.contains(step.token()), "{text}");
            }
            assert_eq!(s.footer_page_number(), Some((1, 2)));
            let cursor = session(&s).cursor;
            key(&mut s, KeyCode::Tab);
            assert_eq!(session(&s).cursor, cursor);
            for page in [2, 1] {
                key(&mut s, KeyCode::BackTab);
                let text = render(&mut s, width, height);
                assert_eq!(s.footer_page_number(), Some((page, 2)));
                assert_eq!(s.body_area, body);
                assert_eq!(s.input_area, Some(input));
                assert_eq!(session(&s).revealed.unwrap().1, description);
                for y in 0..height {
                    for x in 0..width {
                        if matches!(
                            s.pointer.hit(Position::new(x, y)),
                            Some(Hit::Item(Target::WorkflowActions, _))
                        ) {
                            assert!(input.contains(Position::new(x, y)), "{text}");
                        }
                    }
                }
                if page == 2 {
                    assert!(text.contains(i18n::tr!(WorkflowSave)), "{text}");
                    assert!(text.contains(i18n::tr!(WorkflowCancel)), "{text}");
                }
            }
        }
    }
}

#[test]
fn narrow_workflow_actions_reveal_selected_choices_and_escape_returns_to_step_actions() {
    let (_home, mut s) = fixture(vec![]);
    command(&mut s, "schedule actions");
    key(&mut s, KeyCode::BackTab);
    key(&mut s, KeyCode::Right);
    let text = render(&mut s, 12, 9);
    assert!(text.contains(i18n::tr!(WorkflowCancel)), "{text}");
    assert_eq!(s.footer_page_number(), Some((2, 2)));
    key(&mut s, KeyCode::Esc);
    assert_eq!(s.footer_page_number(), Some((1, 2)));
    assert!(matches!(s.mode, Mode::Workflow(_)));
    s.config.mouse.precise_click = false;
    click_item(&mut s, Target::WorkflowToggle, 1);
    assert_eq!(session(&s).steps(), Plan::fetch().steps());
    key(&mut s, KeyCode::Esc);
    assert!(matches!(s.mode, Mode::Browsing));
}

#[test]
fn editor_breadcrumb_has_one_settings_page_and_one_editor_level() {
    let (_home, mut s) = fixture(vec![]);
    for (command_name, section, title) in [(
        "schedule actions",
        Configuration::Schedule,
        i18n::tr!(WorkflowScheduledActions),
    )] {
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
            let navigation = std::iter::repeat_n(KeyCode::Down, 6)
                .chain(std::iter::repeat_n(KeyCode::Up, 6))
                .chain([KeyCode::BackTab, KeyCode::BackTab]);
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
