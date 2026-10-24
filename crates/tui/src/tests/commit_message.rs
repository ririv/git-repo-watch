use super::*;
fn response(cancel: &Cancel) -> anyhow::Result<String> {
    let started = Instant::now();
    while started.elapsed() < Duration::from_millis(120) {
        cancel.check()?;
        thread::sleep(Duration::from_millis(3));
    }
    Ok("feat: generated message".into())
}
fn failed_response(_: &Cancel) -> anyhow::Result<String> {
    anyhow::bail!("mock agent failed")
}

fn setup() -> (support::Fixture, tempfile::TempDir, Screen) {
    let f = support::Fixture::create();
    support::write(f.repo.join("staged.txt"), "staged value\n");
    support::git(&f.repo, &["add", "staged.txt"]);
    let (home, mut s) = fixture(vec![f.repo.clone()]);
    s.commit_generator.response = Some(response);
    (f, home, s)
}
fn wait_generation(s: &mut Screen) {
    let start = Instant::now();
    while s.commit_generator.running() || !s.repository_tasks.is_empty() {
        assert!(start.elapsed() < Duration::from_secs(8));
        s.poll(&Cancel::default());
        thread::sleep(Duration::from_millis(3));
    }
}
#[test]
fn command_generates_an_editable_draft_without_creating_a_commit() {
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let (f, _home, mut s) = setup();
        let head = support::git(&f.repo, &["rev-parse", "HEAD"]);
        command(&mut s, "repo commit generate");
        wait_generation(&mut s);
        let Mode::CommitMessage { editor, .. } = &s.mode else {
            panic!("{:?}", s.mode);
        };
        assert_eq!(editor.text, "feat: generated message");
        assert!(render(&mut s, 100, 28).contains("Ctrl+R"));
        s.paste(" edited");
        key(&mut s, KeyCode::Enter);
        assert!(
            matches!(&s.mode, Mode::CommitConfirm {message, ..} if message.ends_with(" edited"))
        );
        assert_eq!(support::git(&f.repo, &["rev-parse", "HEAD"]), head);
        s.shutdown();
    }
}
#[test]
fn typing_and_returning_during_generation_never_overwrite_user_intent() {
    let (_f, _home, mut s) = setup();
    command(&mut s, "repo commit");
    wait_generation(&mut s);
    s.generate_commit_message();
    assert!(s.commit_generator.running());
    assert!(!matches!(s.mode, Mode::Busy(_)));
    s.paste("manual draft");
    wait_generation(&mut s);
    assert!(matches!(&s.mode, Mode::CommitMessage {editor, ..} if editor.text == "manual draft"));
    assert_eq!(s.message, i18n::tr!(AgentDraftChanged));
    s.generate_commit_message();
    key(&mut s, KeyCode::Esc);
    wait_generation(&mut s);
    assert!(!matches!(s.mode, Mode::CommitMessage { .. }));
    s.shutdown();
}
#[test]
fn failed_generation_retains_draft_and_ctrl_r_cancels() {
    let (_f, _home, mut s) = setup();
    command(&mut s, "repo commit");
    wait_generation(&mut s);
    s.paste("original draft");
    s.commit_generator.response = Some(failed_response);
    s.generate_commit_message();
    wait_generation(&mut s);
    assert!(
        matches!(&s.mode, Mode::Error {origin, ..} if matches!(origin.as_ref(), Mode::CommitMessage {editor, ..} if editor.text == "original draft"))
    );
    key(&mut s, KeyCode::Esc);
    s.commit_generator.response = Some(response);
    s.key(
        KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL),
        &Cancel::default(),
    )
    .unwrap();
    s.key(
        KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL),
        &Cancel::default(),
    )
    .unwrap();
    wait_generation(&mut s);
    assert!(matches!(&s.mode, Mode::CommitMessage {editor, ..} if editor.text == "original draft"));
    assert_eq!(s.message, i18n::tr!(AgentCancelled));
    s.shutdown();
}
#[test]
fn settings_select_agent_and_save_model_and_prompt_in_their_group() {
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let (_home, mut s) = fixture(vec![]);
        command(&mut s, "settings");
        let depth = s.history.len();
        let items = Configuration::General.items(s.snapshot.registration, &s.config.ai);
        assert!(items.contains(&crate::settings::Item::AiEnabled));
        for action in [Action::AiAgent, Action::AiModel, Action::AiPrompt] {
            assert!(items.contains(&crate::settings::Item::Command(action)));
        }
        assert!(!items.contains(&crate::settings::Item::Command(Action::AiSettings)));
        command(&mut s, "settings ai");
        assert_eq!(s.history.len(), depth);
        let text = render(&mut s, 140, 48);
        for label in [
            i18n::tr!(AiEnabled),
            i18n::tr!(AgentChoice),
            i18n::tr!(AgentModel),
            i18n::tr!(AgentPrompt),
        ] {
            assert!(text.contains(label), "{text}");
        }
        assert!(!text.contains(i18n::tr!(AiSettings)));
        assert!(text.contains("Default"));
        s.paste("/model");
        assert!(
            render(&mut s, 140, 48).contains(&format!("[{}: Default]", i18n::tr!(LayoutCurrent)))
        );
        key(&mut s, KeyCode::Esc);
        assert_eq!(s.page, Page::Configuration(Configuration::General));
        assert_eq!(s.command_context(), Some(command::Context::Ai));
        command(&mut s, "settings ai agent claude");
        wait(&mut s);
        assert_eq!(s.config.ai.agent, engine::ai::Agent::Claude);
        command(&mut s, "settings ai model example-model");
        key(&mut s, KeyCode::Enter);
        wait(&mut s);
        assert_eq!(s.config.ai.claude.model, "example-model");
        s.paste("/model");
        assert!(
            render(&mut s, 140, 48)
                .contains(&format!("[{}: example-model]", i18n::tr!(LayoutCurrent)))
        );
        key(&mut s, KeyCode::Esc);
        command(&mut s, "settings ai agent codex");
        wait(&mut s);
        s.paste("/model");
        assert!(
            render(&mut s, 140, 48).contains(&format!("[{}: Default]", i18n::tr!(LayoutCurrent)))
        );
        key(&mut s, KeyCode::Esc);
        command(&mut s, "settings ai agent claude");
        wait(&mut s);
        command(&mut s, "settings ai prompt 使用简洁中文");
        key(&mut s, KeyCode::Enter);
        wait(&mut s);
        assert_eq!(
            s.config.ai.prompt,
            engine::ai::Prompt::Custom {
                text: "使用简洁中文".into()
            }
        );
        assert!(s.config.ai.codex.model.is_empty());
        s.setting_selected = Configuration::General
            .items(s.snapshot.registration, &s.config.ai)
            .len()
            - 1;
        assert!(render(&mut s, 120, 40).contains(i18n::tr!(AgentPrompt)));
        assert!(command::matches("settings ai executable").is_empty());
        s.shutdown();
    }
}
#[test]
fn disabled_ai_is_hidden_and_can_only_be_restored_from_general_settings() {
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let (_f, _home, mut s) = setup();
        command(&mut s, "settings");
        s.activate_setting(crate::settings::Item::AiEnabled, &Cancel::default())
            .unwrap();
        wait(&mut s);
        assert!(!s.config.ai.enabled);
        assert_eq!(s.page, Page::Configuration(Configuration::General));
        let items = Configuration::General.items(s.snapshot.registration, &s.config.ai);
        assert!(items.contains(&crate::settings::Item::AiEnabled));
        for action in [
            Action::AiSettings,
            Action::AiAgent,
            Action::AiModel,
            Action::AiPrompt,
        ] {
            assert!(!items.contains(&crate::settings::Item::Command(action)));
        }
        let text = render(&mut s, 140, 48);
        assert!(text.contains(i18n::tr!(AiEnabled)));
        for label in [
            i18n::tr!(AgentChoice),
            i18n::tr!(AgentModel),
            i18n::tr!(AgentPrompt),
        ] {
            assert!(!text.contains(label), "{text}");
        }
        for text in [
            "settings ai",
            "settings ai agent claude",
            "settings ai model other",
            "settings ai enable",
            "settings ai prompt",
            "settings ai prompt custom",
            "repo commit generate",
        ] {
            command(&mut s, text);
            assert_eq!(s.page, Page::Configuration(Configuration::General));
            assert!(s.bar.commands(s.config.ai.enabled).unwrap().is_empty());
            assert!(!s.config.ai.enabled && !s.commit_generator.running());
            let text = render(&mut s, 140, 40);
            assert!(!text.contains(i18n::tr!(AgentModelHint)));
            assert!(!text.contains(i18n::tr!(AgentPromptHint)));
        }
        key(&mut s, KeyCode::Esc);
        command(&mut s, "repo commit");
        wait_generation(&mut s);
        assert!(matches!(s.mode, Mode::CommitMessage { .. }));
        s.key(
            KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL),
            &Cancel::default(),
        )
        .unwrap();
        assert!(!s.commit_generator.running());
        assert!(!render(&mut s, 120, 32).contains("Ctrl+R"));
        key(&mut s, KeyCode::Esc);
        command(&mut s, "settings");
        s.activate_setting(crate::settings::Item::AiEnabled, &Cancel::default())
            .unwrap();
        wait(&mut s);
        assert!(s.config.ai.enabled);
        command(&mut s, "settings ai");
        assert_eq!(s.page, Page::Configuration(Configuration::General));
        s.shutdown();
    }
}

fn control(s: &mut Screen, ch: char) {
    s.key(
        KeyEvent::new(KeyCode::Char(ch), KeyModifiers::CONTROL),
        &Cancel::default(),
    )
    .unwrap();
}
#[test]
fn complete_prompt_editor_saves_multiline_drafts_and_discards_changes() {
    use engine::ai::Prompt;
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let (_home, mut s) = fixture(vec![]);
        command(&mut s, "settings ai prompt");
        assert!(matches!(s.mode, Mode::PromptEditor(_)));
        assert!(render(&mut s, 100, 28).contains("Write a Git commit subject"));
        control(&mut s, 'e');
        control(&mut s, 'u');
        s.paste("根据暂存内容生成标题。\r\n只输出一行中文。\n");
        assert!(render(&mut s, 44, 16).contains("只输出一行中文"));
        let expected = Prompt::Custom {
            text: "根据暂存内容生成标题。\n只输出一行中文。\n".into(),
        };
        control(&mut s, 's');
        wait(&mut s);
        assert_eq!(s.config.ai.prompt, expected);
        assert_eq!(s.service.store.config().unwrap().ai.prompt, expected);
        assert!(matches!(s.mode, Mode::Browsing));
        command(&mut s, "settings ai prompt");
        s.paste("未保存的修改");
        control(&mut s, 'r');
        let text = render(&mut s, 90, 24);
        assert!(text.contains("未保存的修改"));
        assert!(!text.contains("Ctrl+R"));
        key(&mut s, KeyCode::Esc);
        assert_eq!(s.config.ai.prompt, expected);
        assert_eq!(s.service.store.config().unwrap().ai.prompt, expected);
        s.shutdown();
    }
}
#[test]
fn empty_prompt_is_rejected_without_losing_the_editor_draft() {
    let (_home, mut s) = fixture(vec![]);
    command(&mut s, "settings ai prompt");
    control(&mut s, 'e');
    control(&mut s, 'u');
    control(&mut s, 's');
    assert!(matches!(s.mode, Mode::PromptEditor(_)));
    assert_eq!(s.message, i18n::tr!(AgentPromptInvalid));
    assert_eq!(
        s.service.store.config().unwrap().ai.prompt,
        engine::ai::Prompt::Default
    );
    s.paste("新指令");
    control(&mut s, 's');
    wait(&mut s);
    assert_eq!(
        s.config.ai.prompt,
        engine::ai::Prompt::Custom {
            text: "新指令".into()
        }
    );
    s.shutdown();
}

#[test]
fn agent_is_one_parameter_command_and_completion_never_changes_preferences_before_execution() {
    use crate::command::completion::Menu;
    use engine::ai::Agent;
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let (_home, mut s) = fixture(vec![]);
        command(&mut s, "settings ai");
        let catalog = command::matches("settings ai agent ");
        assert_eq!(catalog.len(), 1);
        assert_eq!(catalog[0].usage(), "settings ai agent <name>");
        s.paste("//settings ai agent ");
        key(&mut s, KeyCode::Tab);
        let bar::InputBar::Command {
            menu: Menu::Values(values),
            ..
        } = &s.bar
        else {
            panic!("argument candidates");
        };
        assert_eq!(values.agents(), Agent::ALL);
        let text = render(&mut s, 120, 28);
        assert!(text.contains(i18n::tr!(ArgumentCandidates)));
        assert!(!text.contains("/settings ai agent claude"));
        key(&mut s, KeyCode::Down);
        key(&mut s, KeyCode::Enter);
        assert!(s.bar.text().ends_with("agent claude"));
        assert_eq!(s.service.store.config().unwrap().ai.agent, Agent::Codex);
        key(&mut s, KeyCode::Enter);
        wait(&mut s);
        assert_eq!(s.config.ai.agent, Agent::Claude);
        s.paste("//settings ai agent CO");
        key(&mut s, KeyCode::Tab);
        assert!(s.bar.text().ends_with("agent codex"));
        assert_eq!(s.service.store.config().unwrap().ai.agent, Agent::Claude);
        key(&mut s, KeyCode::Enter);
        wait(&mut s);
        assert_eq!(s.config.ai.agent, Agent::Codex);
        command(&mut s, "settings ai agent unknown-agent");
        assert!(s.message.contains(i18n::tr!(UnknownAgent)));
        assert_eq!(s.service.store.config().unwrap().ai.agent, Agent::Codex);
        assert!(!matches!(s.mode, Mode::Busy(_)));
        s.shutdown();
    }
}
