use super::*;

#[test]
fn description_selection_completes_real_commands_and_returns_parameter_drafts() {
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let (_home, mut s) = fixture(vec![]);
        let configuration = std::fs::read(s.service.store.config_path()).unwrap();
        s.paste("//检查频率");
        for (width, height) in [(60, 18), (100, 28), (180, 36)] {
            let text = render(&mut s, width, height);
            assert!(text.contains("/schedule interval <duration>"), "{text}");
        }
        key(&mut s, KeyCode::Tab);
        assert_eq!(s.bar.text(), "/schedule interval ");
        s.paste("1w");
        key(&mut s, KeyCode::Enter);
        assert!(matches!(s.mode, Mode::Confirm { .. }));
        key(&mut s, KeyCode::Esc);
        assert_eq!(s.bar.text(), "/schedule interval 1w");
        assert_eq!(
            std::fs::read(s.service.store.config_path()).unwrap(),
            configuration
        );
        assert!(s.service.store.report().unwrap().is_none());
        assert!(s.repository_tasks.is_empty());
    }
}

#[test]
fn clicking_description_results_uses_the_same_selection_and_keeps_frozen_scope() {
    use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
    let (_home, mut s) = fixture(vec!["/example/alpha".into(), "/example/beta".into()]);
    let configuration = std::fs::read(s.service.store.config_path()).unwrap();
    let description =
        i18n::Message::UnregisterRepositoryKeepFilesOnDisk.in_language(i18n::Language::Chinese);
    s.paste(&format!("/{description}"));
    s.select(1);
    render(&mut s, 180, 36);
    let (area, index) = s.command_items[0];
    assert_eq!(s.bar.commands().unwrap()[index].action, Action::Remove);
    s.mouse(
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: area.x,
            row: area.y,
            modifiers: KeyModifiers::NONE,
        },
        &Cancel::default(),
    )
    .unwrap();
    assert!(
        matches!(&s.mode, Mode::Confirm { operation: Confirmation::Remove(path), .. } if path == &PathBuf::from("/example/alpha"))
    );
    key(&mut s, KeyCode::Esc);
    assert_eq!(
        std::fs::read(s.service.store.config_path()).unwrap(),
        configuration
    );
    assert!(s.repository_tasks.is_empty());
}
