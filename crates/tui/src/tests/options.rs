use super::*;
use crate::{
    command::{
        completion::Menu,
        options::{Kind, Value},
    },
    repository_list::{Filter, Order},
};

fn choices(s: &Screen) -> &command::options::Candidates {
    let bar::InputBar::Command {
        menu: Menu::Options(values),
        ..
    } = &s.bar
    else {
        panic!("parameter options expected")
    };
    values
}

#[test]
fn fixed_options_have_one_command_and_automatic_localized_parameter_candidates() {
    for language in [i18n::Language::Chinese, i18n::Language::English] {
        let _language = i18n::using(language);
        let (_home, mut s) = fixture(vec!["/example/a".into()]);
        for (name, kind, count, argument) in [
            (
                "repos filter",
                Kind::Filter,
                7,
                "[all|problems|clean|failed|unchecked|no-upstream|pushes]",
            ),
            ("repos sort", Kind::Sort, 4, "[priority|name|path|updated]"),
            ("repo diff view", Kind::FileView, 2, "[tree|flat]"),
            (
                "repo diff layout",
                Kind::DiffLayout,
                3,
                "[auto|split|unified]",
            ),
            ("search", Kind::Search, 2, "[filter|locate]"),
            (
                "settings interaction",
                Kind::Interaction,
                2,
                "[interactive|text]",
            ),
            (
                "settings startup-check",
                Kind::StartupCheck,
                2,
                "[true|false]",
            ),
            (
                "schedule autopush",
                Kind::Autopush,
                2,
                "[disabled|fast_forward]",
            ),
            ("settings ai agent", Kind::AiAgent, 2, "[codex|claude]"),
        ] {
            assert_eq!(command::matches(name).len(), 1);
            assert_eq!(kind.command().usage(), format!("{name} {argument}"));
            s.paste(&format!("//{name} "));
            assert_eq!(choices(&s).options.len(), count);
            assert_eq!(s.search_commands().unwrap().len(), 1);
            for (width, height) in [(60, 24), (120, 30), (1, 1), (0, 0)] {
                render(&mut s, width, height);
            }
            key(&mut s, KeyCode::Esc);
            key(&mut s, KeyCode::Esc);
        }
        s.paste("//repos filter ");
        assert_eq!(
            choices(&s).options[choices(&s).selected].value,
            Value::Filter(Filter::All)
        );
        let text = render(&mut s, 120, 30);
        assert_eq!(text.matches("/repos filter").count(), 1);
        assert!(text.contains(i18n::tr!(RepositoryFilterFailed)));
        for width in [60, 120] {
            let text = render(&mut s, width, 30);
            let current = text.lines().find(|line| line.contains("│all ")).unwrap();
            assert!(current.ends_with("✓│"), "{current}");
        }
        key(&mut s, KeyCode::Down);
        key(&mut s, KeyCode::Tab);
        assert!(s.bar.text().ends_with("filter problems"));
        assert_eq!(s.filter, Filter::All);
        key(&mut s, KeyCode::Enter);
        assert_eq!(s.filter, Filter::Problems);
        command(&mut s, "repos filter failed");
        assert_eq!(s.filter, Filter::Failed);
        command(&mut s, "repos filter 未配置远程上游");
        assert_eq!(s.filter, Filter::WithoutUpstream);
        command(&mut s, "repos filter invalid-value");
        assert_eq!(s.filter, Filter::WithoutUpstream);
        assert!(choices(&s).options.is_empty());
        assert_eq!(s.message, i18n::tr!(NoMatchingOptions));
        command(&mut s, "repos filter all");
        command(&mut s, "repos sort n");
        assert_eq!(s.order, Order::Name);
        s.shutdown();
    }
}

#[test]
fn choice_hints_follow_available_columns_and_keep_whole_tokens_and_current_values() {
    const FULL: &str = "[all|problems|clean|failed|unchecked|no-upstream|pushes]";
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let (_home, mut s) = fixture(vec!["/example/a".into()]);
        let configuration = std::fs::read(s.service.store.config_path()).unwrap();
        let selection = s.selected_path();
        s.paste("//repos filter");
        for width in [140, 60, 140] {
            let text = render(&mut s, width, 30);
            let row = s.command_items[0].0.y as usize;
            let line = text.lines().nth(row).unwrap();
            assert!(line.contains("/repos filter ["), "{line}");
            let start = line.find('[').unwrap();
            let end = start + line[start..].find(']').unwrap();
            let hint = &line[start..=end];
            if width >= 140 {
                assert_eq!(hint, FULL);
                assert!(
                    line.contains(i18n::CommandSummary::FilterMenu.text()),
                    "{line}"
                );
            } else {
                assert!(hint.ends_with("|…]"), "{line}");
                let visible = hint[1..hint.len() - "|…]".len()].split('|');
                for (actual, expected) in visible.zip(Kind::Filter.options()) {
                    assert_eq!(actual, expected.token);
                }
            }
            assert!(
                line.trim_end_matches(['│', ' '])
                    .ends_with(Filter::All.label()),
                "{line}"
            );
        }
        assert_eq!(s.bar.text(), "/repos filter");
        assert_eq!(s.selected_path(), selection);
        assert_eq!(s.filter, Filter::All);
        assert_eq!(
            std::fs::read(s.service.store.config_path()).unwrap(),
            configuration
        );
        s.shutdown();
    }
}

#[test]
fn value_completion_preserves_plain_search_and_the_selected_repository_until_execution() {
    let (_home, mut s) = fixture(vec!["/example/中文-a".into(), "/example/中文-b".into()]);
    s.paste("中文");
    let selected = s.selected_path();
    let config = std::fs::read(s.service.store.config_path()).unwrap();
    s.paste("//repos sort ");
    key(&mut s, KeyCode::Down);
    key(&mut s, KeyCode::Tab);
    assert_eq!(s.order, Order::Priority);
    assert_eq!(s.selected_path(), selected);
    key(&mut s, KeyCode::Enter);
    assert_eq!(s.order, Order::Name);
    assert_eq!(s.bar.text(), "中文");
    assert_eq!(s.selected_path(), selected);
    assert_eq!(
        std::fs::read(s.service.store.config_path()).unwrap(),
        config
    );
    s.shutdown();
}

#[test]
fn persisted_choices_complete_without_writing_and_keep_review_cancellation_and_mouse_selection() {
    use command::ConfigArgument;
    use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let (_home, mut s) = fixture(vec![]);
        s.config.check_on_startup = true;
        s.config.push_policy = PushPolicy::Disabled;
        s.service.store.save_config(&s.config).unwrap();
        let before = std::fs::read(s.service.store.config_path()).unwrap();
        command(&mut s, "settings startup-check");
        assert_eq!(choices(&s).options.len(), 2);
        assert_eq!(choices(&s).current, Some(Value::StartupCheck(true)));
        let text = render(&mut s, 100, 28);
        assert!(text.contains(i18n::tr!(StartupCheckDisabledOption)));
        key(&mut s, KeyCode::Down);
        key(&mut s, KeyCode::Tab);
        assert_eq!(s.bar.text(), "/settings startup-check false");
        assert_eq!(
            std::fs::read(s.service.store.config_path()).unwrap(),
            before
        );
        key(&mut s, KeyCode::Enter);
        assert!(matches!(&s.mode, Mode::Confirm {
            operation: Confirmation::Configure { command: ConfigArgument::StartupCheck, value, .. }, ..
        } if value == "false"));
        assert_eq!(
            std::fs::read(s.service.store.config_path()).unwrap(),
            before
        );
        key(&mut s, KeyCode::Esc);
        assert_eq!(s.bar.text(), "/settings startup-check false");
        key(&mut s, KeyCode::Enter);
        key(&mut s, KeyCode::Enter);
        wait(&mut s);
        assert!(!s.config.check_on_startup);
        assert!(!s.service.store.config().unwrap().check_on_startup);
        assert!(s.check_task.is_none());
        assert!(s.service.store.report().unwrap().is_none());

        let saved = std::fs::read(s.service.store.config_path()).unwrap();
        command(&mut s, "settings startup-check");
        render(&mut s, 100, 28);
        let row = s
            .command_items
            .iter()
            .find(|(_, index)| *index == 0)
            .unwrap()
            .0;
        s.mouse(
            MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: row.x,
                row: row.y,
                modifiers: KeyModifiers::NONE,
            },
            &Cancel::default(),
        )
        .unwrap();
        assert!(matches!(&s.mode, Mode::Confirm {
            operation: Confirmation::Configure { command: ConfigArgument::StartupCheck, value, .. }, ..
        } if value == "true"));
        key(&mut s, KeyCode::Esc);
        assert_eq!(std::fs::read(s.service.store.config_path()).unwrap(), saved);

        command(&mut s, "schedule autopush");
        assert_eq!(choices(&s).options.len(), 2);
        assert_eq!(
            choices(&s).current,
            Some(Value::Autopush(PushPolicy::Disabled))
        );
        key(&mut s, KeyCode::Down);
        key(&mut s, KeyCode::Tab);
        assert_eq!(s.bar.text(), "/schedule autopush fast_forward");
        assert_eq!(std::fs::read(s.service.store.config_path()).unwrap(), saved);
        key(&mut s, KeyCode::Enter);
        assert!(matches!(&s.mode, Mode::Confirm {
            operation: Confirmation::Configure { command: ConfigArgument::Autopush, value, .. }, ..
        } if value == "fast_forward"));
        key(&mut s, KeyCode::Esc);
        assert_eq!(s.config.push_policy, PushPolicy::Disabled);
        assert_eq!(std::fs::read(s.service.store.config_path()).unwrap(), saved);
        command(&mut s, "schedule autopush invalid");
        assert_eq!(s.message, i18n::tr!(NoMatchingOptions));
        assert_eq!(std::fs::read(s.service.store.config_path()).unwrap(), saved);
        s.shutdown();
    }
}
