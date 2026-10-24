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
    assert_eq!(
        s.bar.commands(s.config.ai.enabled).unwrap()[index].action,
        Action::Remove
    );
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

#[test]
fn concise_commands_show_values_without_scope_or_detail_rows_and_remain_clickable() {
    use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let (_home, mut s) = fixture(vec![]);
        command(&mut s, "settings ai");
        let configuration = std::fs::read(s.service.store.config_path()).unwrap();
        s.paste("/");
        let text = render(&mut s, 140, 32);
        let items = s.command_items.clone();
        assert_eq!(items.len(), 3);
        for (area, index) in &items {
            let command = s.bar.commands(s.config.ai.enabled).unwrap()[*index];
            assert_eq!(area.height, 1);
            let line = text.lines().nth(area.y.into()).unwrap();
            assert!(line.contains(command.summary().text()), "{line}");
            for absent in [
                i18n::tr!(Global),
                i18n::tr!(CurrentRepo),
                i18n::tr!(CurrentView),
                i18n::tr!(LayoutCurrent),
                "[",
                "]",
            ] {
                assert!(!line.contains(absent), "{line}");
            }
        }
        assert!(
            items
                .windows(2)
                .all(|pair| pair[0].0.bottom() == pair[1].0.y)
        );
        assert!(
            text.lines()
                .nth(items[0].0.y.into())
                .unwrap()
                .contains("Codex")
        );
        assert!(
            text.lines()
                .nth(items[1].0.y.into())
                .unwrap()
                .contains("Default")
        );
        assert!(!text.contains(i18n::tr!(AgentNameHint)));
        key(&mut s, KeyCode::Down);
        let text = render(&mut s, 140, 32);
        assert_eq!(s.command_items, items);
        assert!(text.contains(i18n::CommandSummary::AiModel.text()));
        let model = items[1].0;
        assert!(
            s.mouse(
                MouseEvent {
                    kind: MouseEventKind::Down(MouseButton::Left),
                    column: model.x + 2,
                    row: model.y,
                    modifiers: KeyModifiers::NONE,
                },
                &Cancel::default()
            )
            .unwrap()
        );
        assert_eq!(s.bar.text(), "/settings ai model ");
        assert!(matches!(s.mode, Mode::Browsing));
        assert_eq!(
            std::fs::read(s.service.store.config_path()).unwrap(),
            configuration
        );
        s.shutdown();
    }
}

#[test]
fn command_descriptions_remain_searchable_across_interface_languages() {
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let (_home, mut s) = fixture(vec![]);
        for query in ["填写模型名", "ENTER A MODEL NAME", "设置模型", "SET MODEL"] {
            s.paste(&format!("//{query}"));
            let matches = s.bar.commands(s.config.ai.enabled).unwrap();
            assert_eq!(matches.len(), 1);
            assert_eq!(matches[0].action, Action::AiModel);
            let text = render(&mut s, 140, 28);
            assert!(text.contains(i18n::CommandSummary::AiModel.text()));
            key(&mut s, KeyCode::Esc);
        }
        s.shutdown();
    }
}
