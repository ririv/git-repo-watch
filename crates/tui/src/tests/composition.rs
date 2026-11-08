use super::*;
use crate::composition::{Focus, Session, State};
fn setup() -> (support::Fixture, tempfile::TempDir, Screen) {
    let f = support::Fixture::create();
    let text = (0..40).map(|i| format!("line {i}\n")).collect::<String>();
    f.commit("file 中文.txt", &text);
    support::write(
        f.repo.join("file 中文.txt"),
        &text
            .replace("line 2\n", "topic one\n")
            .replace("line 32\n", "topic two\n"),
    );
    let snapshot = engine::composition::capture(&f.repo, &Cancel::default()).unwrap();
    assert_eq!(snapshot.units.len(), 2);
    let commits = snapshot
        .units
        .iter()
        .map(|unit| engine::composition::Step {
            message: format!("feat: {}\n\n说明正文", unit.id),
            units: vec![unit.id.clone()],
        })
        .collect();
    let prepared = engine::composition::Prepared {
        snapshot,
        plan: engine::composition::Plan { commits },
        completed: vec![],
    };
    let (home, mut s) = fixture(vec![f.repo.clone()]);
    s.mode = Mode::Composition(Box::new(Session::new(prepared)));
    s.sync_bar();
    (f, home, s)
}
fn control(s: &mut Screen, ch: char) {
    s.key(
        KeyEvent::new(KeyCode::Char(ch), KeyModifiers::CONTROL),
        &Cancel::default(),
    )
    .unwrap();
}
fn begin_waiting(s: &mut Screen) -> std::sync::mpsc::Sender<()> {
    let Mode::Composition(session) = s.take_mode() else {
        panic!()
    };
    let prepared = session.prepared.clone();
    let path = prepared.snapshot.path.clone();
    s.mode = Mode::CompositionWaiting(Box::new(crate::composition::waiting::Session::new(
        path.clone(),
        Box::new(Mode::Composition(session)),
    )));
    s.sync_bar();
    let (send, receive) = std::sync::mpsc::channel();
    s.start_repository(
        path,
        i18n::tr!(ComposeGenerating),
        Effect::ReadOnly,
        move |_, _, _| {
            receive.recv_timeout(Duration::from_secs(3))?;
            Ok(Outcome::Composed(Box::new(prepared)))
        },
    );
    send
}
#[test]
fn compose_enters_the_generation_page_immediately_and_keeps_failure_in_place() {
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let f = support::Fixture::create();
        let (_home, mut s) = fixture(vec![f.repo.clone()]);
        command(&mut s, "repo compose");
        assert!(
            matches!(&s.mode, Mode::CompositionWaiting(waiting) if matches!(waiting.state, crate::composition::waiting::State::Running(_)) && waiting.path == f.repo)
        );
        for (width, height) in [(60, 24), (160, 32), (24, 9), (1, 1), (0, 0)] {
            let text = render(&mut s, width, height);
            if width >= 60 {
                assert_eq!(text.matches(i18n::tr!(ComposeGenerating)).count(), 1);
            }
        }
        wait(&mut s);
        assert!(
            matches!(&s.mode, Mode::CompositionWaiting(waiting) if matches!(&waiting.state, crate::composition::waiting::State::Failed { detail, .. } if detail.contains(i18n::tr!(ComposeNoChanges))))
        );
        assert!(render(&mut s, 160, 32).contains(i18n::tr!(ComposeGenerationFailed)));
        key(&mut s, KeyCode::Enter);
        assert!(
            matches!(&s.mode, Mode::CompositionWaiting(waiting) if matches!(waiting.state, crate::composition::waiting::State::Running(_)))
        );
        wait(&mut s);
        key(&mut s, KeyCode::Esc);
        assert!(matches!(s.mode, Mode::Browsing));
        s.shutdown();
    }
}
#[test]
fn generation_completes_despite_navigation_keys_and_saved_waiting_pages_receive_results() {
    for leave in [false, true] {
        let (_f, _home, mut s) = setup();
        let gate = begin_waiting(&mut s);
        key(&mut s, KeyCode::Down);
        if leave {
            s.open_page(Page::Help);
            assert!(matches!(s.mode, Mode::Browsing));
        }
        gate.send(()).unwrap();
        wait(&mut s);
        if leave {
            assert_eq!(s.page, Page::Help);
            assert!(matches!(s.mode, Mode::Browsing));
            s.return_to_view();
        }
        assert!(
            matches!(&s.mode, Mode::Composition(session) if session.prepared.plan.commits.len() == 2)
        );
        s.shutdown();
    }
}
#[test]
fn escaping_generation_cancels_its_worker_and_restores_the_previous_plan() {
    let (_f, _home, mut s) = setup();
    let gate = begin_waiting(&mut s);
    let task = s.repository_tasks.values().next().unwrap().cancel.clone();
    key(&mut s, KeyCode::Esc);
    assert!(task.is_cancelled());
    assert!(
        matches!(&s.mode, Mode::Composition(session) if session.prepared.plan.commits.len() == 2)
    );
    gate.send(()).unwrap();
    wait(&mut s);
    assert!(
        matches!(&s.mode, Mode::Composition(session) if session.prepared.plan.commits.len() == 2)
    );
    s.shutdown();
}
#[test]
fn composition_has_its_own_context_and_editable_messages_without_changing_repository_contents() {
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let (f, _home, mut s) = setup();
        let head = support::git(&f.repo, &["rev-parse", "HEAD"]);
        assert_eq!(s.command_context(), Some(command::Context::Composition));
        for (width, height) in [(60, 24), (160, 32), (24, 9), (1, 1), (0, 0), (100, 28)] {
            render(&mut s, width, height);
        }
        s.paste("/");
        assert!(s.bar.text().contains("/repo compose"));
        let commands = s.search_commands().unwrap();
        assert!(
            commands
                .iter()
                .any(|command| command.name == "repo compose commit")
        );
        assert!(
            !commands
                .iter()
                .any(|command| command.name == "repo commit generate")
        );
        key(&mut s, KeyCode::Esc);
        command(&mut s, "repo compose message");
        assert!(
            matches!(&s.mode, Mode::Composition(session) if matches!(session.state, State::Editing(_)))
        );
        for width in [60, 160] {
            let text = render(&mut s, width, 32);
            assert!(text.contains(i18n::tr!(ComposePending)));
            assert!(text.contains(i18n::tr!(ComposeUnits)));
            assert!(text.contains("feat: W2"));
            assert!(text.contains("file 中文.txt"));
        }
        for (width, height) in [(24, 9), (1, 1), (0, 0)] {
            render(&mut s, width, height);
        }
        control(&mut s, 'e');
        key(&mut s, KeyCode::Enter);
        s.paste("补充");
        control(&mut s, 's');
        assert!(
            matches!(&s.mode, Mode::Composition(session) if session.selected_message().ends_with("补充") && matches!(session.state, State::Browsing))
        );
        assert_eq!(support::git(&f.repo, &["rev-parse", "HEAD"]), head);
        s.shutdown();
    }
}
#[test]
fn inline_message_scrolling_and_cancellation_preserve_the_selected_plan_and_patch() {
    use crossterm::event::{MouseEvent, MouseEventKind};
    let (_f, _home, mut s) = setup();
    let original = (0..30)
        .map(|line| format!("正文 {line}\n"))
        .collect::<String>();
    if let Mode::Composition(session) = &mut s.mode {
        session.prepared.plan.commits[0].message = original.clone();
    }
    render(&mut s, 160, 40);
    let (_, detail) = view::composition::areas(s.body_area.unwrap());
    let (message, _, _) = view::composition::detail_areas(detail);
    let wheel = MouseEvent {
        kind: MouseEventKind::ScrollDown,
        column: message.x + 2,
        row: message.y + 1,
        modifiers: KeyModifiers::NONE,
    };
    assert!(s.mouse(wheel, &Cancel::default()).unwrap());
    let Mode::Composition(session) = &s.mode else {
        panic!()
    };
    assert_eq!(session.message_scroll, 3);
    assert_eq!(session.scroll, 0);
    key(&mut s, KeyCode::Enter);
    render(&mut s, 160, 40);
    assert!(s.mouse(wheel, &Cancel::default()).unwrap());
    assert!(
        matches!(&s.mode, Mode::Composition(session) if matches!(&session.state, State::Editing(editor) if editor.scroll == 3) && session.scroll == 0)
    );
    s.paste("临时修改");
    key(&mut s, KeyCode::Esc);
    let Mode::Composition(session) = &s.mode else {
        panic!()
    };
    assert_eq!(session.selected_message(), original);
    assert_eq!(session.selected, 0);
    assert_eq!(session.unit, 0);
    assert_eq!(session.message_scroll, 3);
    assert_eq!(session.scroll, 0);
    s.shutdown();
}
#[test]
fn plan_order_merge_split_and_reassignment_keep_every_unit_once() {
    let (_f, _home, mut s) = setup();
    command(&mut s, "repo compose reorder down");
    assert!(matches!(&s.mode, Mode::Composition(session) if session.selected == 1));
    command(&mut s, "repo compose reorder up");
    command(&mut s, "repo compose merge");
    assert!(
        matches!(&s.mode, Mode::Composition(session) if session.prepared.plan.commits.len() == 1 && session.ids().len() == 2)
    );
    s.cycle_pane(false);
    command(&mut s, "repo compose split");
    assert!(
        matches!(&s.mode, Mode::Composition(session) if session.prepared.plan.commits.len() == 2)
    );
    command(&mut s, "repo compose move");
    key(&mut s, KeyCode::Up);
    key(&mut s, KeyCode::Enter);
    let Mode::Composition(session) = &s.mode else {
        panic!()
    };
    assert_eq!(session.prepared.plan.commits.len(), 1);
    assert_eq!(session.ids().len(), 2);
    session.prepared.validate().unwrap();
    s.shutdown();
}
#[test]
fn each_confirmed_step_commits_only_its_units_and_retains_the_rest_of_the_plan() {
    let (f, _home, mut s) = setup();
    command(&mut s, "repo compose commit");
    assert!(
        matches!(&s.mode, Mode::Composition(session) if matches!(session.state, State::Confirm(_)))
    );
    key(&mut s, KeyCode::Right);
    key(&mut s, KeyCode::Enter);
    assert!(s.repository_tasks.is_empty());
    command(&mut s, "repo compose commit");
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    let Mode::Composition(session) = &s.mode else {
        panic!("{:?}", s.mode)
    };
    assert_eq!(session.prepared.plan.commits.len(), 1);
    assert_eq!(session.prepared.completed.len(), 1);
    assert!(support::git(&f.repo, &["diff"]).contains("topic two"));
    assert!(support::git(&f.repo, &["diff", "--cached"]).is_empty());
    command(&mut s, "repo compose commit");
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    assert!(matches!(s.mode, Mode::Browsing));
    assert!(support::git(&f.repo, &["status", "--porcelain"]).is_empty());
    s.shutdown();
}
#[test]
fn unit_mouse_hits_and_global_focus_mode_select_the_existing_plan_panels() {
    use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
    let (_f, _home, mut s) = setup();
    render(&mut s, 160, 32);
    let (_, detail) = view::composition::areas(s.body_area.unwrap());
    let (_, units, _) = view::composition::detail_areas(detail);
    assert!(
        s.mouse(
            MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: units.x + 2,
                row: units.y + 1,
                modifiers: KeyModifiers::NONE
            },
            &Cancel::default()
        )
        .unwrap()
    );
    assert!(matches!(&s.mode, Mode::Composition(session) if session.focus == Focus::Units));
    command(&mut s, "focus");
    key(&mut s, KeyCode::Left);
    key(&mut s, KeyCode::Enter);
    assert!(matches!(&s.mode, Mode::Composition(session) if session.focus == Focus::Commits));
    s.shutdown();
}
#[test]
fn disabled_ai_hides_every_composition_entry() {
    let (_home, mut s) = fixture(vec![PathBuf::from("/example/repository")]);
    s.config.ai.enabled = false;
    s.paste("/repo compose");
    assert!(s.search_commands().unwrap().is_empty());
    for command in command::COMMANDS
        .iter()
        .filter(|command| command.name.starts_with("repo compose"))
    {
        assert!(!command.action.available(&s.config));
    }
    s.shutdown();
}

#[test]
fn reorder_options_explain_boundaries_and_only_rearrange_the_draft_plan() {
    let (f, _home, mut s) = setup();
    let before_head = support::git(&f.repo, &["rev-parse", "HEAD"]);
    let before_worktree = support::git(&f.repo, &["diff"]);
    let before_index = std::fs::read(f.repo.join(".git/index")).unwrap();
    fn steps(s: &Screen) -> Vec<(String, Vec<String>)> {
        let Mode::Composition(session) = &s.mode else {
            unreachable!()
        };
        session
            .prepared
            .plan
            .commits
            .iter()
            .map(|step| (step.message.clone(), step.units.clone()))
            .collect()
    }
    let before_plan = steps(&s);
    command(&mut s, "repo compose reorder");
    assert_eq!(super::options::choices(&s).current, None);
    let text = render(&mut s, 140, 32);
    assert!(text.contains(i18n::tr!(ComposeFirstStep)), "{text}");
    key(&mut s, KeyCode::Enter);
    assert_eq!(s.message, i18n::tr!(ComposeFirstStep));
    key(&mut s, KeyCode::Down);
    key(&mut s, KeyCode::Tab);
    assert!(matches!(&s.mode, Mode::Composition(session) if session.selected == 0));
    key(&mut s, KeyCode::Enter);
    assert!(matches!(&s.mode, Mode::Composition(session) if session.selected == 1));
    assert_eq!(
        steps(&s),
        vec![before_plan[1].clone(), before_plan[0].clone()]
    );
    command(&mut s, "repo compose reorder");
    let text = render(&mut s, 140, 32);
    assert!(text.contains(i18n::tr!(ComposeLastStep)), "{text}");
    key(&mut s, KeyCode::Down);
    key(&mut s, KeyCode::Enter);
    assert_eq!(s.message, i18n::tr!(ComposeLastStep));
    key(&mut s, KeyCode::Esc);
    key(&mut s, KeyCode::Esc);
    command(&mut s, "repo compose reorder up");
    assert_eq!(steps(&s), before_plan);
    let Mode::Composition(session) = &s.mode else {
        unreachable!()
    };
    session.prepared.validate().unwrap();
    assert_eq!(support::git(&f.repo, &["rev-parse", "HEAD"]), before_head);
    assert_eq!(support::git(&f.repo, &["diff"]), before_worktree);
    assert_eq!(
        std::fs::read(f.repo.join(".git/index")).unwrap(),
        before_index
    );
    s.shutdown();
}

#[test]
fn experimental_toggle_hides_the_whole_group_and_saved_pages_but_keeps_ai_messages() {
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let (_f, _home, mut s) = setup();
        assert!(s.config.experimental_features);
        let title = i18n::tr!(ComposeTitle);
        assert!(render(&mut s, 160, 40).contains(title));
        let mut title_terminal = Terminal::new(TestBackend::new(160, 40)).unwrap();
        title_terminal
            .draw(|frame| view::render(frame, &mut s))
            .unwrap();
        let title_colors = title_terminal
            .backend()
            .buffer()
            .content
            .iter()
            .filter(|cell| cell.symbol() != " " && cell.fg == view::EXPERIMENTAL)
            .count();
        assert!(title_colors > 0);
        s.paste("//repo compose");
        let menu = render(&mut s, 160, 40);
        let label = command::COMMANDS
            .iter()
            .find(|entry| entry.name == "repo compose")
            .unwrap()
            .summary()
            .text();
        assert!(menu.contains(label));
        assert!(label.contains("AI"));
        assert!(!label.contains("实验性") && !label.contains("experimental"));
        let mut terminal = Terminal::new(TestBackend::new(160, 40)).unwrap();
        terminal.draw(|frame| view::render(frame, &mut s)).unwrap();
        let menu_colors = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .filter(|cell| cell.symbol() != " " && cell.fg == view::EXPERIMENTAL)
            .count();
        assert!(menu_colors > title_colors);
        key(&mut s, KeyCode::Esc);
        command(&mut s, "settings");
        let item = crate::settings::Item::ExperimentalFeatures;
        let items = Configuration::General.items(s.snapshot.registration, &s.config.ai);
        s.setting_selected = items.iter().position(|entry| *entry == item).unwrap();
        assert!(render(&mut s, 140, 50).contains(i18n::tr!(ExperimentalFeatures)));
        s.activate_setting(item, &Cancel::default()).unwrap();
        wait(&mut s);
        assert!(!s.config.experimental_features);
        assert!(!s.service.store.config().unwrap().experimental_features);
        assert!(s.config.ai.enabled);
        assert!(Action::CommitGenerate.available(&s.config));
        for entry in command::COMMANDS
            .iter()
            .filter(|entry| entry.action.experimental())
        {
            assert!(!entry.action.available(&s.config));
        }
        s.return_to_view();
        assert!(!matches!(
            s.mode,
            Mode::Composition(_) | Mode::CompositionWaiting(_)
        ));
        s.paste("//repo compose");
        assert!(s.search_commands().unwrap().is_empty());
        key(&mut s, KeyCode::Esc);
        s.compose_action(crate::composition::Action::Open, s.selected_path());
        assert!(s.repository_tasks.is_empty());
        assert!(!matches!(
            s.mode,
            Mode::Composition(_) | Mode::CompositionWaiting(_)
        ));
        s.activate_setting(item, &Cancel::default()).unwrap();
        wait(&mut s);
        assert!(s.service.store.config().unwrap().experimental_features);
        assert!(
            command::COMMANDS
                .iter()
                .filter(|entry| entry.action.experimental())
                .all(|entry| entry.action.available(&s.config))
        );
        s.shutdown();
    }
}

#[test]
fn disabling_experiments_during_generation_does_not_reopen_the_composition_page() {
    let (_f, _home, mut s) = setup();
    let release = begin_waiting(&mut s);
    s.service
        .configure(engine::service::ConfigEdit::ExperimentalFeatures(false))
        .unwrap();
    s.config = s.service.store.config().unwrap();
    release.send(()).unwrap();
    wait(&mut s);
    assert!(!matches!(
        s.mode,
        Mode::Composition(_) | Mode::CompositionWaiting(_)
    ));
    assert!(s.config.ai.enabled);
    s.shutdown();
}
