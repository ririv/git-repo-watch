use super::*;
use crate::history::menu::{Operation, PointerInput};

fn select(s: &mut Screen, oid: &str) {
    let Mode::History(view) = &mut s.mode else {
        panic!("history");
    };
    let crate::history::View::Graph(session) = view.as_mut() else {
        panic!("graph");
    };
    session.state.select(
        session
            .records
            .iter()
            .position(|r| r.entry.oid.to_string() == oid),
    );
}
fn mark(s: &mut Screen, oid: &str) {
    select(s, oid);
    key(s, KeyCode::Char(' '));
}
fn choose(s: &mut Screen, operation: Operation) {
    let index = s
        .history_actions()
        .unwrap()
        .operations()
        .iter()
        .position(|item| *item == operation)
        .unwrap();
    key(s, KeyCode::Home);
    for _ in 0..index {
        key(s, KeyCode::Down);
    }
    key(s, KeyCode::Enter);
}
fn marks(s: &Screen) -> Vec<String> {
    let Mode::History(view) = &s.mode else {
        panic!("history");
    };
    view.session()
        .picks()
        .iter()
        .map(ToString::to_string)
        .collect()
}

#[test]
fn enter_opens_actions_without_preparing_a_mutation_and_cancel_preserves_marks_and_search() {
    let f = support::Fixture::create();
    f.commit("one.txt", "one\n");
    let first = support::git(&f.repo, &["rev-parse", "HEAD"]);
    f.commit("two.txt", "two\n");
    let second = support::git(&f.repo, &["rev-parse", "HEAD"]);
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo log");
    wait(&mut s);
    command(&mut s, "repo log select");
    mark(&mut s, &first);
    mark(&mut s, &second);
    s.paste("txt");
    let task_count = s.service.store.tasks(200).unwrap().len();
    key(&mut s, KeyCode::Enter);
    assert!(s.history_actions().is_some());
    assert_eq!(s.service.store.tasks(200).unwrap().len(), task_count);
    assert_eq!(support::git(&f.repo, &["rev-parse", "HEAD"]), second);
    assert!(
        !s.history_actions()
            .unwrap()
            .operations()
            .iter()
            .any(|op| matches!(op, Operation::Revert(_)))
    );
    for language in [i18n::Language::Chinese, i18n::Language::English] {
        let _language = i18n::using(language);
        let text = render(&mut s, 120, 30);
        assert!(text.contains(i18n::tr!(HistoryActionsTitle)), "{text}");
        assert!(text.contains(i18n::tr!(HistoryActionCherryPick)), "{text}");
        assert!(text.contains(i18n::tr!(HistoryActionCopy)), "{text}");
        assert!(!text.contains(i18n::tr!(HistoryActionRevert)), "{text}");
        let input = s.input_area.unwrap();
        let last = s.history_actions().unwrap().operations().len() - 1;
        let point = ratatui::layout::Position::new(input.right() - 2, input.y - 1);
        assert!(
            matches!(s.pointer.hit(point), Some(crate::pointer::Hit::Button(crate::pointer::Action::HistoryMenu(PointerInput::Choose(index)))) if index == last)
        );
        for (width, height) in [(80, 18), (40, 10), (1, 1)] {
            render(&mut s, width, height);
        }
    }
    s.paste("//quit");
    assert_eq!(s.bar.editor().text, "/quit");
    key(&mut s, KeyCode::Esc);
    assert!(s.history_actions().is_none());
    assert_eq!(marks(&s), [first.clone(), second.clone()]);
    assert_eq!(s.bar.editor().text, "txt");
    key(&mut s, KeyCode::Enter);
    choose(&mut s, Operation::Clear);
    assert!(marks(&s).is_empty());
    assert!(render(&mut s, 120, 30).contains("│[ ]"));
    key(&mut s, KeyCode::Esc);
    mark(&mut s, &first);
    key(&mut s, KeyCode::Enter);
    choose(&mut s, Operation::Done);
    assert!(marks(&s).is_empty());
    assert!(!render(&mut s, 120, 30).contains("[ ]"));
    s.shutdown();
}

#[test]
fn a_single_marked_commit_is_reverted_even_when_the_cursor_is_on_another_commit() {
    let f = support::Fixture::create();
    f.commit("remove.txt", "remove\n");
    let marked = support::git(&f.repo, &["rev-parse", "HEAD"]);
    f.commit("keep.txt", "keep\n");
    let before = support::git(&f.repo, &["rev-parse", "HEAD"]);
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo log");
    wait(&mut s);
    command(&mut s, "repo log select");
    mark(&mut s, &marked);
    select(&mut s, &before);
    key(&mut s, KeyCode::Enter);
    choose(&mut s, Operation::Revert(marked.parse().unwrap()));
    wait(&mut s);
    assert!(
        matches!(&s.mode, Mode::Confirm { operation: Confirmation::Mutation { plan, .. }, .. }
        if matches!(plan.as_ref(), engine::operations::mutation::Plan::Replay(engine::operations::replay::Plan::Start(start))
            if start.candidate.commit.to_string() == marked && start.candidate.kind == engine::operations::replay::Kind::Revert))
    );
    assert_eq!(support::git(&f.repo, &["rev-parse", "HEAD"]), before);
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    assert!(!f.repo.join("remove.txt").exists());
    assert!(f.repo.join("keep.txt").exists());
    assert_eq!(support::git(&f.repo, &["rev-parse", "HEAD~1"]), before);
    s.shutdown();
}

#[test]
fn menu_wheel_and_pointer_follow_preferences_and_copy_only_the_marked_hashes() {
    let f = support::Fixture::create();
    f.commit("one.txt", "one\n");
    let first = support::git(&f.repo, &["rev-parse", "HEAD"]);
    f.commit("two.txt", "two\n");
    let second = support::git(&f.repo, &["rev-parse", "HEAD"]);
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    let (tx, rx) = mpsc::channel();
    s.clipboard = crate::clipboard::Writer::with_sink(move |text| {
        tx.send(text)?;
        Ok(())
    });
    key(&mut s, KeyCode::Enter);
    wait_workspace(&mut s);
    s.workspace.session_mut().unwrap().selection = crate::workspace::Section::History;
    key(&mut s, KeyCode::Enter);
    command(&mut s, "repo log select");
    mark(&mut s, &first);
    mark(&mut s, &second);
    key(&mut s, KeyCode::Enter);
    let copy_index = s
        .history_actions()
        .unwrap()
        .operations()
        .iter()
        .position(|op| *op == Operation::Copy)
        .unwrap();
    let wheel = crossterm::event::MouseEvent {
        kind: crossterm::event::MouseEventKind::ScrollDown,
        column: 20,
        row: 10,
        modifiers: KeyModifiers::NONE,
    };
    s.config.mouse.scroll = false;
    s.mouse(wheel, &Cancel::default()).unwrap();
    assert_eq!(s.history_actions().unwrap().state.selected(), Some(0));
    s.config.mouse.scroll = true;
    s.mouse(wheel, &Cancel::default()).unwrap();
    assert_eq!(
        s.history_actions().unwrap().state.selected(),
        Some(copy_index)
    );
    render(&mut s, 120, 30);
    let point=(0..30).flat_map(|y|(0..120).map(move|x|ratatui::layout::Position::new(x,y))).find(|p|matches!(s.pointer.hit(*p),Some(crate::pointer::Hit::Button(crate::pointer::Action::HistoryMenu(PointerInput::Choose(index)))) if index == copy_index)).unwrap();
    let click = crossterm::event::MouseEvent {
        kind: crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left),
        column: point.x,
        row: point.y,
        modifiers: KeyModifiers::NONE,
    };
    s.config.mouse.precise_click = false;
    s.mouse(click, &Cancel::default()).unwrap();
    assert!(s.history_actions().is_some());
    s.config.mouse.precise_click = true;
    s.mouse(click, &Cancel::default()).unwrap();
    assert!(s.history_actions().is_none());
    assert_eq!(
        rx.recv_timeout(Duration::from_secs(8)).unwrap(),
        format!("{first}\n{second}")
    );
    assert_eq!(marks(&s), [first, second.clone()]);
    assert_eq!(support::git(&f.repo, &["rev-parse", "HEAD"]), second);
    s.shutdown();
}
