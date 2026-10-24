use super::*;
use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};

#[test]
fn results_wrap_below_the_input_hint_without_separating_the_command_menu() {
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let (_home, mut s) = fixture(vec![]);
        s.paste("/settings");
        s.message = "result line one\n结果第二行".into();
        for width in [40, 60, 110] {
            let text = render(&mut s, width, 30);
            let input = s.input_area.unwrap();
            assert_eq!(
                s.command_items.last().unwrap().0.bottom() + 1,
                input.y,
                "{text}"
            );
            assert!(
                text.lines()
                    .nth(input.bottom().into())
                    .unwrap()
                    .contains("Tab"),
                "{text}"
            );
            assert!(
                text.lines()
                    .nth(usize::from(input.bottom() + 1))
                    .unwrap()
                    .contains("result line one"),
                "{text}"
            );
            assert!(
                text.lines()
                    .nth(usize::from(input.bottom() + 2))
                    .unwrap()
                    .contains("结果第二行"),
                "{text}"
            );
        }
        s.message.clear();
        let blank = render(&mut s, 60, 30);
        let empty_layout = (s.input_area, s.command_items.clone());
        assert_eq!(s.input_area.unwrap().bottom(), 28);
        assert!(blank.lines().last().unwrap().trim().is_empty());
        s.message = "operation complete".into();
        assert!(
            render(&mut s, 60, 30)
                .lines()
                .last()
                .unwrap()
                .contains("operation complete")
        );
        assert_eq!((s.input_area, s.command_items.clone()), empty_layout);
        s.message.clear();
        render(&mut s, 60, 30);
        assert_eq!((s.input_area, s.command_items.clone()), empty_layout);
        assert!(matches!(
            s.feedback,
            crate::feedback::Feedback::Inline { expansion: None }
        ));
    }
}

#[test]
fn long_result_reader_preserves_the_full_text_and_original_command_draft() {
    let (_home, mut s) = fixture(vec!["/example/alpha".into()]);
    let before_config = std::fs::read(s.service.store.config_path()).unwrap();
    s.paste("/settings");
    s.message = format!(
        "{}\ncomplete result tail",
        "中文 result with spaces\n".repeat(80)
    );
    let original_message = s.message.clone();
    let draft = s.bar.text();
    let text = render(&mut s, 60, 24);
    assert!(text.contains(i18n::tr!(ExpandOperationResult)));
    let expansion = match s.feedback {
        crate::feedback::Feedback::Inline {
            expansion: Some(area),
        } => area,
        _ => panic!("long result can be expanded"),
    };
    key_with(&mut s, KeyCode::Enter, KeyModifiers::ALT);
    render(&mut s, 60, 24);
    assert!(s.input_area.is_none() && s.command_items.is_empty());
    s.paste("//quit");
    key(&mut s, KeyCode::Char('x'));
    assert_eq!(s.bar.text(), draft);
    key(&mut s, KeyCode::End);
    assert!(render(&mut s, 60, 24).contains("complete result tail"));
    s.message = "new result while reading".into();
    assert!(render(&mut s, 60, 24).contains("complete result tail"));
    key(&mut s, KeyCode::Esc);
    assert_eq!(s.bar.text(), draft);
    s.message = original_message;
    render(&mut s, 60, 24);
    s.config.mouse.precise_click = false;
    let click = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: expansion.x,
        row: expansion.y,
        modifiers: KeyModifiers::NONE,
    };
    assert!(!s.mouse(click, &Cancel::default()).unwrap());
    s.config.mouse.precise_click = true;
    assert!(s.mouse(click, &Cancel::default()).unwrap());
    render(&mut s, 60, 24);
    let point = match &s.feedback {
        crate::feedback::Feedback::Reading(reader) => reader.content,
        _ => panic!("reading"),
    };
    let wheel = MouseEvent {
        kind: MouseEventKind::ScrollDown,
        column: point.x,
        row: point.y,
        modifiers: KeyModifiers::NONE,
    };
    s.mouse(wheel, &Cancel::default()).unwrap();
    assert!(matches!(&s.feedback, crate::feedback::Feedback::Reading(reader) if reader.scroll > 0));
    s.config.mouse.scroll = false;
    let before = match &s.feedback {
        crate::feedback::Feedback::Reading(reader) => reader.scroll,
        _ => unreachable!(),
    };
    s.mouse(wheel, &Cancel::default()).unwrap();
    assert!(
        matches!(&s.feedback, crate::feedback::Feedback::Reading(reader) if reader.scroll == before)
    );
    for (width, height) in [(1, 1), (0, 0), (12, 6), (40, 12), (80, 24)] {
        render(&mut s, width, height);
    }
    key(&mut s, KeyCode::Enter);
    render(&mut s, 60, 24);
    assert_eq!(s.bar.text(), draft);
    assert_eq!(
        std::fs::read(s.service.store.config_path()).unwrap(),
        before_config
    );
    assert!(s.repository_tasks.is_empty() && s.check_task.is_none());
}

#[test]
fn alt_enter_without_an_expandable_result_never_executes_a_command() {
    let (_home, mut s) = fixture(vec![]);
    s.paste("/settings");
    render(&mut s, 60, 24);
    key_with(&mut s, KeyCode::Enter, KeyModifiers::ALT);
    assert_eq!(s.page, Page::Repositories);
    assert_eq!(s.bar.text(), "/settings");
}

fn key_with(s: &mut Screen, code: KeyCode, modifiers: KeyModifiers) {
    s.key(
        crossterm::event::KeyEvent::new(code, modifiers),
        &Cancel::default(),
    )
    .unwrap();
}
