use super::*;
fn response(
    cancel: &Cancel,
    _: &mut dyn FnMut(engine::commit_message::Update),
) -> anyhow::Result<String> {
    let started = Instant::now();
    while started.elapsed() < Duration::from_millis(120) {
        cancel.check()?;
        thread::sleep(Duration::from_millis(3));
    }
    Ok("feat: generated message".into())
}
fn failed_response(
    _: &Cancel,
    _: &mut dyn FnMut(engine::commit_message::Update),
) -> anyhow::Result<String> {
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
        assert_eq!(editor.text(), "feat: generated message");
        assert!(render(&mut s, 100, 28).contains("Ctrl+R"));
        s.paste(" edited");
        assert!(
            matches!(&s.mode, Mode::CommitMessage {editor, ..} if editor.focused() && editor.text().ends_with(" edited"))
        );
        assert_eq!(support::git(&f.repo, &["rev-parse", "HEAD"]), head);
        s.shutdown();
    }
}
#[test]
fn generation_locks_typing_paste_and_review_and_cancellation_restores_original() {
    let (_f, _home, mut s) = setup();
    command(&mut s, "repo commit");
    wait_generation(&mut s);
    s.paste("original draft");
    s.generate_commit_message();
    assert!(s.commit_generator.running() && !matches!(s.mode, Mode::Busy(_)));
    s.paste("must not enter the message");
    key(&mut s, KeyCode::Char('x'));
    key(&mut s, KeyCode::Enter);
    control(&mut s, 's');
    assert!(
        matches!(&s.mode,Mode::CommitMessage { editor,.. } if editor.generating() && editor.text().is_empty())
    );
    key(&mut s, KeyCode::Esc);
    wait_generation(&mut s);
    assert!(
        matches!(&s.mode,Mode::CommitMessage { editor,.. } if !editor.generating() && editor.text()=="original draft")
    );
    assert_eq!(s.message, i18n::tr!(AgentCancelled));
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
        matches!(&s.mode, Mode::Error {origin, ..} if matches!(origin.as_ref(), Mode::CommitMessage {editor, ..} if editor.text() == "original draft"))
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
    assert!(
        matches!(&s.mode, Mode::CommitMessage {editor, ..} if editor.text() == "original draft")
    );
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
            render(&mut s, 140, 48)
                .lines()
                .any(|line| line.contains("model [<model>|default]") && line.contains("Default"))
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
                .lines()
                .any(|line| line.contains("model [<model>|default]")
                    && line.contains("example-model"))
        );
        key(&mut s, KeyCode::Esc);
        command(&mut s, "settings ai agent codex");
        wait(&mut s);
        s.paste("/model");
        assert!(
            render(&mut s, 140, 48)
                .lines()
                .any(|line| line.contains("model [<model>|default]") && line.contains("Default"))
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
            assert!(s.search_commands().unwrap().is_empty());
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
        assert!(render(&mut s, 100, 28).contains("Write a Git commit message"));
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
        assert_eq!(catalog[0].usage(), "settings ai agent [codex|claude]");
        s.paste("//settings ai agent ");
        key(&mut s, KeyCode::Tab);
        let bar::InputBar::Command {
            menu: Menu::Options(values),
            ..
        } = &s.bar
        else {
            panic!("argument candidates");
        };
        assert_eq!(
            values
                .options
                .iter()
                .map(|option| option.value)
                .collect::<Vec<_>>(),
            Agent::ALL
                .iter()
                .copied()
                .map(command::options::Value::AiAgent)
                .collect::<Vec<_>>()
        );
        let text = render(&mut s, 120, 28);
        assert!(!text.contains(i18n::tr!(ArgumentCandidates)));
        assert!(text.contains("codex") && text.contains("claude"));
        assert!(!text.contains("/settings ai agent claude"));
        key(&mut s, KeyCode::Down);
        key(&mut s, KeyCode::Tab);
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
        assert_eq!(s.message, i18n::tr!(NoMatchingOptions));
        assert_eq!(s.service.store.config().unwrap().ai.agent, Agent::Codex);
        assert!(!matches!(s.mode, Mode::Busy(_)));
        s.shutdown();
    }
}

fn multiline_response(
    _: &Cancel,
    _: &mut dyn FnMut(engine::commit_message::Update),
) -> anyhow::Result<String> {
    Ok("feat: support message bodies\n\n解释为什么需要修改。\n保留分段与中文。".into())
}
#[test]
fn generated_body_is_editable_and_committed_intact_without_an_extra_preview() {
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let (f, _home, mut s) = setup();
        s.commit_generator.response = Some(multiline_response);
        command(&mut s, "repo commit generate");
        wait_generation(&mut s);
        for (width, height) in [(100, 28), (44, 18), (24, 9), (1, 1), (0, 0), (100, 28)] {
            render(&mut s, width, height);
            assert!(
                matches!(&s.mode, Mode::CommitMessage { editor, .. } if editor.text() == multiline_response(&Cancel::default(), &mut |_| {}).unwrap())
            );
        }
        let text = render(&mut s, 100, 28);
        assert!(text.contains("support message bodies") && text.contains("解释为什么需要修改"));
        assert!(text.contains("Ctrl+S"));
        let input_height = s.input_area.unwrap().height;
        // 正文光标移动不改变审核区和消息框的分配高度。
        key(&mut s, KeyCode::Down);
        key(&mut s, KeyCode::Down);
        s.paste("新增：");
        control(&mut s, 'e');
        key(&mut s, KeyCode::Enter);
        s.paste("最后一行");
        let expected = "feat: support message bodies\n\n新增：解释为什么需要修改。\n保留分段与中文。\n最后一行";
        render(&mut s, 100, 28);
        assert_eq!(s.input_area.unwrap().height, input_height);
        let text = render(&mut s, 100, 28);
        assert!(text.contains("新增：解释为什么需要修改。") && text.contains("最后一行"));
        assert!(matches!(&s.mode, Mode::CommitMessage { editor, .. } if editor.text() == expected));
        control(&mut s, 's');
        assert!(matches!(&s.mode, Mode::CommitMessage { editor, .. } if editor.submitting()));
        wait_generation(&mut s);
        assert_eq!(
            support::git(&f.repo, &["log", "-1", "--format=%B"]),
            expected
        );
        s.shutdown();
    }
}

#[test]
fn manual_message_paste_keeps_line_breaks_even_without_ai() {
    let (f, _home, mut s) = setup();
    s.config.ai.enabled = false;
    command(&mut s, "repo commit");
    wait_generation(&mut s);
    s.paste("fix: 标题\r\n\r\n第一段\r第二段");
    let text = render(&mut s, 100, 28);
    assert!(text.contains("第一段") && text.contains("第二段") && !text.contains("Ctrl+R"));
    control(&mut s, 's');
    wait_generation(&mut s);
    assert_eq!(
        support::git(&f.repo, &["log", "-1", "--format=%B"]),
        "fix: 标题\n\n第一段\n第二段"
    );
    s.shutdown();
}

fn streaming_response(
    cancel: &Cancel,
    emit: &mut dyn FnMut(engine::commit_message::Update),
) -> anyhow::Result<String> {
    emit(engine::commit_message::Update::Append(
        "feat: partial stream".into(),
    ));
    let started = Instant::now();
    while started.elapsed() < Duration::from_millis(500) {
        cancel.check()?;
        thread::sleep(Duration::from_millis(3));
    }
    Ok("feat: completed stream\n\n完整正文。".into())
}
#[test]
fn streaming_preview_and_horizontal_actions_are_separate_and_cancellation_restores_the_draft() {
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let (f, _home, mut s) = setup();
        command(&mut s, "repo commit");
        wait_generation(&mut s);
        s.paste("original message");
        s.commit_generator.response = Some(streaming_response);
        s.generate_commit_message();
        let start = Instant::now();
        while !matches!(&s.mode,Mode::CommitMessage { editor,.. } if editor.text().contains("partial stream"))
        {
            assert!(start.elapsed() < Duration::from_secs(3));
            s.poll(&Cancel::default());
            thread::sleep(Duration::from_millis(3));
        }
        let head = support::git(&f.repo, &["rev-parse", "HEAD"]);
        assert!(s.commit_generator.running());
        let text = render(&mut s, 120, 32);
        assert!(text.contains("partial stream") && text.contains(i18n::tr!(CommitPreview)));
        assert_eq!(s.input_area.unwrap().height, 3);
        s.paste("forbidden");
        control(&mut s, 's');
        assert!(
            matches!(&s.mode,Mode::CommitMessage { editor,.. } if editor.generating() && !editor.text().contains("forbidden"))
        );
        assert!(text.contains(i18n::tr!(CommitCancelAction)));
        let actions = text
            .lines()
            .nth(s.input_area.unwrap().y as usize + 1)
            .unwrap();
        assert!(!actions.contains(i18n::tr!(ActionCommit)));
        assert!(!actions.contains(i18n::tr!(CommitRegenerateAction)));
        key(&mut s, KeyCode::Enter);
        wait_generation(&mut s);
        assert!(
            matches!(&s.mode,Mode::CommitMessage { editor,.. } if !editor.generating() && editor.text()=="original message")
        );
        assert_eq!(support::git(&f.repo, &["rev-parse", "HEAD"]), head);
        // 说明中的路径和 / 保持正文；操作栏不再接受命令草稿。
        s.paste(" /literal/path");
        key(&mut s, KeyCode::Enter);
        s.paste("正文新行");
        key(&mut s, KeyCode::Tab);
        s.paste("/review");
        key(&mut s, KeyCode::Char('/'));
        assert!(s.bar.is_plain());
        assert!(
            matches!(&s.mode,Mode::CommitMessage { editor,.. } if editor.text()=="original message /literal/path\n正文新行")
        );
        key(&mut s, KeyCode::Enter);
        assert!(
            matches!(&s.mode,Mode::CommitMessage { editor,.. } if editor.submitting() && editor.text()=="original message /literal/path\n正文新行")
        );
        wait_generation(&mut s);
        assert_eq!(
            support::git(&f.repo, &["log", "-1", "--format=%B"]),
            "original message /literal/path\n正文新行"
        );
        s.shutdown();
    }
}

#[test]
fn commit_review_stays_above_the_message_and_actions_follow_the_editor_at_every_width() {
    use crate::commit_form::{Action, EditAction};
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let (_f, _home, mut s) = setup();
        command(&mut s, "repo commit");
        wait_generation(&mut s);
        s.paste("feat: message below staged changes\n\n正文");
        for (width, height) in [(60, 24), (100, 28), (160, 32)] {
            let text = render(&mut s, width, height);
            assert!(text.contains("Ctrl+S") && text.contains("Ctrl+R"), "{text}");
            let (message, review) = view::commit::areas(s.body_area.unwrap());
            assert_eq!(message.y, review.bottom());
            assert_eq!(message.x, review.x);
            assert_eq!(message.width, review.width);
            assert_eq!(message.bottom(), s.input_area.unwrap().y);
            let title_row = text
                .lines()
                .position(|line| line.contains(i18n::tr!(CommitPreview)))
                .unwrap();
            let message_row = text
                .lines()
                .position(|line| line.contains("feat: message below"))
                .unwrap();
            assert!(message_row > title_row, "{text}");
            let options = view::commit::options(
                view::input::content_area(s.input_area.unwrap()),
                match &s.mode {
                    Mode::CommitMessage { editor, .. } => editor,
                    _ => unreachable!(),
                },
                true,
            );
            assert_eq!(
                options
                    .iter()
                    .map(|(_, action)| *action)
                    .collect::<Vec<_>>(),
                [
                    Action::Edit(EditAction::Commit),
                    Action::Edit(EditAction::Return),
                    Action::Edit(EditAction::Regenerate)
                ]
            );
            assert!(
                options
                    .iter()
                    .all(|(area, _)| area.y == options[0].0.y && !area.is_empty())
            );
            assert!(
                text.contains(i18n::tr!(ActionCommit))
                    && text.contains(i18n::tr!(CommitRegenerateAction))
            );
        }
        s.shutdown();
    }
}

#[test]
fn horizontal_actions_support_keyboard_navigation_and_hide_generation_when_ai_is_disabled() {
    use crate::commit_form::{Action, EditAction};
    let (f, _home, mut s) = setup();
    s.config.ai.enabled = false;
    command(&mut s, "repo commit");
    wait_generation(&mut s);
    s.paste("fix: manual message\n\n正文");
    let selected = |s: &Screen| match &s.mode {
        Mode::CommitMessage { editor, .. } => editor.selected(),
        _ => panic!("{:?}", s.mode),
    };
    let text = render(&mut s, 100, 28);
    assert!(!text.contains(i18n::tr!(CommitRegenerateAction)));
    key(&mut s, KeyCode::Tab);
    assert_eq!(selected(&s), Some(Action::Edit(EditAction::Commit)));
    key(&mut s, KeyCode::Right);
    assert_eq!(selected(&s), Some(Action::Edit(EditAction::Return)));
    key(&mut s, KeyCode::Left);
    assert_eq!(selected(&s), Some(Action::Edit(EditAction::Commit)));
    key(&mut s, KeyCode::BackTab);
    assert_eq!(selected(&s), None);
    key(&mut s, KeyCode::BackTab);
    assert_eq!(selected(&s), Some(Action::Edit(EditAction::Return)));
    key(&mut s, KeyCode::Tab);
    assert_eq!(selected(&s), None);
    key(&mut s, KeyCode::Tab);
    key(&mut s, KeyCode::Tab);
    assert_eq!(selected(&s), Some(Action::Edit(EditAction::Return)));
    key(&mut s, KeyCode::Up);
    assert_eq!(selected(&s), None);
    s.paste("保留");
    s.config.ai.enabled = true;
    key(&mut s, KeyCode::Tab);
    key(&mut s, KeyCode::Tab);
    assert_eq!(selected(&s), Some(Action::Edit(EditAction::Return)));
    key(&mut s, KeyCode::Tab);
    assert_eq!(selected(&s), Some(Action::Edit(EditAction::Regenerate)));
    key(&mut s, KeyCode::Enter);
    assert!(s.commit_generator.running());
    assert_eq!(selected(&s), Some(Action::CancelGeneration));
    key(&mut s, KeyCode::Enter);
    wait_generation(&mut s);
    assert!(
        matches!(&s.mode, Mode::CommitMessage { editor, .. } if editor.text().ends_with("正文保留") && editor.focused())
    );
    key(&mut s, KeyCode::Tab);
    key(&mut s, KeyCode::Right);
    key(&mut s, KeyCode::Enter);
    assert!(matches!(s.mode, Mode::Browsing));
    assert_eq!(
        support::git(&f.repo, &["log", "-1", "--format=%s"]),
        "initial"
    );
    s.shutdown();
}

#[test]
fn horizontal_action_clicks_select_and_double_clicks_match_enter() {
    use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
    let (f, _home, mut s) = setup();
    command(&mut s, "repo commit");
    wait_generation(&mut s);
    s.paste("fix: clicked commit");
    render(&mut s, 100, 28);
    let Mode::CommitMessage { editor, .. } = &s.mode else {
        unreachable!()
    };
    let (area, action) = view::commit::options(
        view::input::content_area(s.input_area.unwrap()),
        editor,
        true,
    )[0];
    let click = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: area.x + 1,
        row: area.y,
        modifiers: KeyModifiers::NONE,
    };
    s.config.mouse.precise_click = false;
    assert!(!s.mouse(click, &Cancel::default()).unwrap());
    s.config.mouse.precise_click = true;
    assert!(s.mouse(click, &Cancel::default()).unwrap());
    assert!(
        matches!(&s.mode, Mode::CommitMessage { editor, .. } if editor.selected() == Some(action))
    );
    assert!(s.mouse(click, &Cancel::default()).unwrap());
    assert!(
        matches!(&s.mode, Mode::CommitMessage { editor, .. } if editor.submitting() && editor.text() == "fix: clicked commit")
    );
    wait_generation(&mut s);
    assert_eq!(
        support::git(&f.repo, &["log", "-1", "--format=%B"]),
        "fix: clicked commit"
    );
    s.shutdown();
}

#[test]
fn a_changed_index_rejects_direct_commit_and_returns_an_editable_draft() {
    let (f, _home, mut s) = setup();
    command(&mut s, "repo commit");
    wait_generation(&mut s);
    s.paste("fix: preserved message\n\n完整正文");
    support::write(f.repo.join("staged.txt"), "changed after preview\n");
    support::git(&f.repo, &["add", "staged.txt"]);
    key(&mut s, KeyCode::Tab);
    key(&mut s, KeyCode::Enter);
    assert!(matches!(&s.mode, Mode::CommitMessage { editor, .. } if editor.submitting()));
    key(&mut s, KeyCode::Tab);
    wait_generation(&mut s);
    assert!(
        matches!(&s.mode, Mode::Error { origin, .. } if matches!(origin.as_ref(),
        Mode::CommitMessage { editor, .. } if editor.focused() && editor.text() == "fix: preserved message\n\n完整正文"))
    );
    assert_eq!(
        support::git(&f.repo, &["log", "-1", "--format=%s"]),
        "initial"
    );
    key(&mut s, KeyCode::Esc);
    s.paste("追加");
    assert!(
        matches!(&s.mode, Mode::CommitMessage { editor, .. } if editor.text().ends_with("完整正文追加"))
    );
    s.shutdown();
}

#[test]
#[cfg(unix)]
fn submitting_locks_the_snapshot_and_ignored_editing_keys_do_not_block_completion() {
    use std::os::unix::fs::PermissionsExt;
    struct Release(PathBuf);
    impl Drop for Release {
        fn drop(&mut self) {
            let _ = std::fs::write(&self.0, "release");
        }
    }
    let (f, _home, mut s) = setup();
    let hook = f.repo.join(".git/hooks/pre-commit");
    support::write(
        &hook,
        "#!/bin/sh\ntouch .git/commit-started\nwhile [ ! -f .git/commit-release ]; do sleep 0.01; done\n",
    );
    std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o755)).unwrap();
    let release = Release(f.repo.join(".git/commit-release"));
    command(&mut s, "repo commit");
    wait_generation(&mut s);
    s.paste("feat: confirmed snapshot\n\n正文");
    key(&mut s, KeyCode::Tab);
    key(&mut s, KeyCode::Enter);
    let start = Instant::now();
    while !f.repo.join(".git/commit-started").exists() {
        assert!(start.elapsed() < Duration::from_secs(8));
        thread::sleep(Duration::from_millis(3));
    }
    s.paste("forbidden edit");
    key(&mut s, KeyCode::Char('x'));
    key(&mut s, KeyCode::Tab);
    key(&mut s, KeyCode::Up);
    control(&mut s, 's');
    control(&mut s, 'r');
    assert!(!s.commit_generator.running());
    assert_eq!(s.repository_tasks.len(), 1);
    assert!(
        matches!(&s.mode, Mode::CommitMessage { editor, .. } if editor.submitting() && editor.text() == "feat: confirmed snapshot\n\n正文")
    );
    let text = render(&mut s, 100, 28);
    assert!(text.contains(i18n::tr!(CommitSubmitting)));
    assert!(!text.contains(i18n::tr!(CommitRegenerateAction)));
    drop(release);
    wait_generation(&mut s);
    assert!(matches!(s.mode, Mode::Browsing));
    assert_eq!(
        support::git(&f.repo, &["log", "-1", "--format=%B"]),
        "feat: confirmed snapshot\n\n正文"
    );
    s.shutdown();
}

#[test]
fn narrow_action_bars_keep_the_selected_action_visible_without_adding_rows() {
    use crate::commit_form::EditAction;
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let mut draft = crate::commit_form::Draft::new(String::new());
        for width in [1, 12, 24, 44] {
            for action in [
                EditAction::Commit,
                EditAction::Return,
                EditAction::Regenerate,
            ] {
                draft.focus(crate::commit_form::Focus::Actions(action));
                let bounds = ratatui::layout::Rect::new(2, 3, width, 1);
                let options = view::commit::options(bounds, &draft, true);
                let (area, _) = options
                    .iter()
                    .find(|(_, action)| Some(*action) == draft.selected())
                    .unwrap();
                // 一列宽时也保留当前按钮的可点击区域。
                assert!(!area.is_empty());
                assert!(options.iter().all(|(area, _)| area.y == bounds.y
                    && area.height == 1
                    && area.intersection(bounds) == *area));
            }
        }
    }
}

#[test]
fn unrelated_error_while_generating_restores_the_draft_before_saving_its_origin() {
    let (_f, _home, mut s) = setup();
    command(&mut s, "repo commit");
    wait_generation(&mut s);
    s.paste("preserved draft");
    s.commit_generator.response = Some(streaming_response);
    s.generate_commit_message();
    s.error("unrelated operation", anyhow::anyhow!("failure"));
    wait_generation(&mut s);
    assert!(
        matches!(&s.mode,Mode::Error { origin,.. } if matches!(origin.as_ref(),Mode::CommitMessage { editor,.. } if editor.text()=="preserved draft" && !editor.generating()))
    );
    key(&mut s, KeyCode::Esc);
    s.paste(" edited");
    assert!(
        matches!(&s.mode,Mode::CommitMessage { editor,.. } if editor.text()=="preserved draft edited")
    );
    s.shutdown();
}
