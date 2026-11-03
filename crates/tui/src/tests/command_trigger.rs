use super::*;
use crate::{bar::InputBar, command::Context};

#[test]
fn ideographic_comma_obeys_locale_and_preference_for_typing_and_paste() {
    for language in [i18n::Language::Chinese, i18n::Language::English] {
        let _language = i18n::using(language);
        for enabled in [true, false] {
            for paste in [true, false] {
                let (_home, mut s) = fixture(vec![]);
                s.config.chinese_command_trigger = enabled;
                if paste {
                    s.paste("、");
                } else {
                    key(&mut s, KeyCode::Char('、'));
                }
                let active = enabled && language == i18n::Language::Chinese;
                assert_eq!(matches!(s.bar, InputBar::Command { .. }), active);
                assert_eq!(s.bar.text(), if active { "/" } else { "、" });
                key(&mut s, KeyCode::Esc);
                key(&mut s, KeyCode::Char('/'));
                assert!(matches!(s.bar, InputBar::Command { .. }));
            }
        }
    }
}

#[test]
fn alias_retains_scope_supports_global_escape_and_replaces_selected_prefix() {
    let _language = i18n::using(i18n::Language::Chinese);
    let (_home, mut s) = fixture(vec![]);
    for context in [
        Context::Repository,
        Context::Diff(crate::command::context::DiffContext::Worktree),
        Context::History,
        Context::Branches,
        Context::Stash,
        Context::Settings,
        Context::Schedule,
        Context::Repositories,
    ] {
        s.bar = InputBar::Command {
            input: command::Input::new(Some(context)),
            selected: 0,
            menu: Default::default(),
            path: None,
            selection: crate::clipboard::Selection::None,
        };
        key(&mut s, KeyCode::Char('、'));
        assert_eq!(s.bar.text(), "/");
    }
    command(&mut s, "settings");
    key(&mut s, KeyCode::Char('、'));
    assert_eq!(s.bar.text(), "/settings ");
    key(&mut s, KeyCode::Backspace);
    assert!(s.bar.command_input().unwrap().prefix_selected());
    key(&mut s, KeyCode::Char('、'));
    assert_eq!(s.bar.text(), "/");
    key(&mut s, KeyCode::Esc);
    for input in ["、、help", "/、help", "、/help"] {
        s.paste(input);
        assert_eq!(s.bar.text(), "/help");
        key(&mut s, KeyCode::Esc);
    }
    s.paste("、startup enabled false");
    assert_eq!(s.bar.text(), "/settings startup enabled false");
    key(&mut s, KeyCode::Esc);
    key(&mut s, KeyCode::Esc);
    key(&mut s, KeyCode::Char('、'));
    key(&mut s, KeyCode::Backspace);
    s.paste("、help");
    assert_eq!(s.bar.text(), "/help");
}

#[test]
fn punctuation_inside_arguments_and_search_is_not_rewritten() {
    let _language = i18n::using(i18n::Language::Chinese);
    let (_home, mut s) = fixture(vec![]);
    s.paste("、repos add /example/中文、目录");
    assert_eq!(s.bar.text(), "/repos add /example/中文、目录");
    key(&mut s, KeyCode::Char('、'));
    s.paste("、后缀");
    assert_eq!(s.bar.text(), "/repos add /example/中文、目录、、后缀");
    key(&mut s, KeyCode::Esc);
    let f = support::Fixture::create();
    support::write(f.repo.join("中文、目录.txt"), "content");
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    open_files(&mut s);
    s.paste("中文");
    key(&mut s, KeyCode::Char('、'));
    assert_eq!(s.bar.text(), "中文、");
    assert!(matches!(s.bar, InputBar::Search(_)));
    key(&mut s, KeyCode::Home);
    key(&mut s, KeyCode::Char('、'));
    assert_eq!(s.bar.text(), "/repo diff ");
}

fn trigger_index(s: &Screen) -> usize {
    Configuration::General
        .items(s.snapshot.registration, &s.config.ai)
        .iter()
        .position(|item| *item == crate::settings::Item::ChineseCommandTrigger)
        .unwrap()
}
fn select_trigger(s: &mut Screen) {
    for _ in 0..trigger_index(s) {
        key(s, KeyCode::Down);
    }
}

#[test]
fn chinese_trigger_is_a_page_only_toggle_that_persists_without_confirmation() {
    for language in [i18n::Language::Chinese, i18n::Language::English] {
        let _language = i18n::using(language);
        let (_home, mut s) = fixture(vec![]);
        command(&mut s, "settings");
        let visible = language == i18n::Language::Chinese;
        if visible {
            select_trigger(&mut s);
        }
        for (width, height) in [(60, 18), (120, 30)] {
            assert_eq!(
                render(&mut s, width, height).contains(i18n::tr!(ChineseCommandTrigger)),
                visible
            );
        }
        key(&mut s, KeyCode::Char('/'));
        let names = s
            .search_commands()
            .unwrap()
            .iter()
            .map(|c| c.name)
            .collect::<Vec<_>>();
        assert_eq!(
            names,
            [
                "settings concurrency",
                "settings startup",
                "settings wrap",
                "settings interaction",
                "settings ai",
                "settings startup enabled",
                "settings ai agent",
                "settings ai model",
                "settings ai prompt",
            ]
        );
        assert!(command::argument_input("settings chinese-command-trigger false").is_none());
        assert!(command::matches("settings chinese-command-trigger").is_empty());
        if !visible {
            continue;
        }
        key(&mut s, KeyCode::Esc);
        for expected in [false, true] {
            key(&mut s, KeyCode::Enter);
            assert!(matches!(s.mode, Mode::Busy(_)));
            assert!(matches!(s.bar, InputBar::CommandsOnly));
            wait(&mut s);
            assert!(matches!(s.mode, Mode::Browsing));
            assert_eq!(s.page, Page::Configuration(Configuration::General));
            assert_eq!(s.setting_selected, trigger_index(&s));
            assert_eq!(s.config.chinese_command_trigger, expected);
            assert_eq!(
                s.service.store.config().unwrap().chinese_command_trigger,
                expected
            );
            key(&mut s, KeyCode::Char('、'));
            assert_eq!(matches!(s.bar, InputBar::Command { .. }), expected);
            key(&mut s, KeyCode::Esc);
            // 关闭别名时，普通文字沿用搜索行为；重新打开设置验证持久化值。
            command(&mut s, "settings");
            select_trigger(&mut s);
        }
    }
}

#[test]
fn failed_toggle_save_keeps_the_previous_value_and_returns_to_the_setting() {
    let _language = i18n::using(i18n::Language::Chinese);
    let (_home, mut s) = fixture(vec![]);
    command(&mut s, "settings");
    select_trigger(&mut s);
    let path = s.service.store.config_path();
    let text = std::fs::read_to_string(&path).unwrap();
    let invalid = format!("{text}unknown_key = true\n");
    std::fs::write(&path, &invalid).unwrap();
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    assert!(matches!(s.mode, Mode::Error { .. }));
    assert!(s.config.chinese_command_trigger);
    assert_eq!(std::fs::read_to_string(&path).unwrap(), invalid);
    std::fs::write(path, text).unwrap();
    key(&mut s, KeyCode::Esc);
    assert!(matches!(s.mode, Mode::Browsing));
    assert_eq!(s.page, Page::Configuration(Configuration::General));
    assert_eq!(s.setting_selected, trigger_index(&s));
}

#[test]
fn commit_message_keeps_typed_and_pasted_punctuation() {
    let _language = i18n::using(i18n::Language::Chinese);
    let f = support::Fixture::create();
    support::write(f.repo.join("change.txt"), "content");
    support::git(&f.repo, &["add", "change.txt"]);
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo commit");
    wait(&mut s);
    key(&mut s, KeyCode::Char('、'));
    s.paste("、正文");
    assert!(matches!(&s.mode, Mode::CommitMessage { editor, .. } if editor.text() == "、、正文"));
}
