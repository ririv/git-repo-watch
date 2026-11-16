use super::*;
use engine::operations::{
    mutation,
    replay::{self, State},
};

fn select(s: &mut Screen, oid: &str) {
    let Mode::History(view) = &mut s.mode else {
        panic!("history");
    };
    let crate::history::View::Graph(graph) = view.as_mut() else {
        panic!("graph");
    };
    graph.state.select(
        graph
            .records
            .iter()
            .position(|record| record.entry.oid.to_string() == oid),
    );
}

fn marked(s: &Screen) -> Vec<String> {
    let Mode::History(view) = &s.mode else {
        panic!("history");
    };
    view.session()
        .picks()
        .iter()
        .map(ToString::to_string)
        .collect()
}

fn batch(s: &Screen) -> &replay::Batch {
    let Mode::Confirm {
        operation: Confirmation::Mutation { plan, .. },
        ..
    } = &s.mode
    else {
        panic!("review: {:?}", s.mode);
    };
    let mutation::Plan::Replay(replay::Plan::Batch(batch)) = plan.as_ref() else {
        panic!("batch");
    };
    batch
}

#[test]
fn history_selection_is_explicit_and_exiting_restores_the_ordinary_list() {
    let f = support::Fixture::create();
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo log");
    wait(&mut s);
    let ordinary = render(&mut s, 120, 30);
    assert!(!ordinary.contains("[ ]"), "{ordinary}");
    assert!(!ordinary.contains(i18n::tr!(HistoryMarked)), "{ordinary}");
    assert!(
        !(0..30)
            .flat_map(|y| (0..120).map(move |x| ratatui::layout::Position::new(x, y)))
            .any(|point| matches!(
                s.pointer.hit(point),
                Some(crate::pointer::Hit::Button(
                    crate::pointer::Action::ToggleCommit(_)
                ))
            ))
    );
    key(&mut s, KeyCode::Insert);
    assert!(marked(&s).is_empty());
    assert!(!render(&mut s, 120, 30).contains("[ ]"));
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    assert!(
        matches!(&s.mode, Mode::History(view) if matches!(view.as_ref(), crate::history::View::Commit(_)))
    );
    key(&mut s, KeyCode::Esc);
    command(&mut s, "repo log select");
    let choosing = render(&mut s, 120, 30);
    assert!(choosing.contains("│[ ]"), "{choosing}");
    assert!(
        choosing.contains(i18n::tr!(HistorySelectionHint)),
        "{choosing}"
    );
    key(&mut s, KeyCode::Enter);
    assert!(
        matches!(&s.mode, Mode::History(view) if matches!(view.as_ref(), crate::history::View::Graph(_)))
    );
    assert_eq!(s.message, i18n::tr!(HistorySelectionRequired));
    key(&mut s, KeyCode::Insert);
    assert_eq!(marked(&s).len(), 1);
    command(&mut s, "repo log select");
    assert!(marked(&s).is_empty());
    assert!(!render(&mut s, 120, 30).contains("[ ]"));
    command(&mut s, "repo log select");
    key(&mut s, KeyCode::Insert);
    key(&mut s, KeyCode::Esc);
    assert!(
        matches!(&s.mode, Mode::History(view) if matches!(view.as_ref(), crate::history::View::Graph(_)))
    );
    assert!(marked(&s).is_empty());
    assert!(!render(&mut s, 120, 30).contains("[ ]"));
    s.shutdown();
}

#[test]
fn selected_count_remains_visible_while_graphics_load_or_are_unavailable() {
    let f = support::Fixture::create();
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo log");
    wait(&mut s);
    key(&mut s, KeyCode::Insert);
    assert!(marked(&s).is_empty());
    command(&mut s, "repo log select");
    key(&mut s, KeyCode::Insert);
    for language in [i18n::Language::Chinese, i18n::Language::English] {
        let _language = i18n::using(language);
        for support in [
            crate::graphics::Support::Pixels(crate::graphics::Backend::test()),
            crate::graphics::Support::Unavailable(i18n::tr!(GraphicsUnsupported).into()),
        ] {
            s.graphics = crate::graphics::Display::new(support);
            for (width, height) in [(120, 30), (60, 18)] {
                let text = render(&mut s, width, height);
                assert!(
                    text.contains(&format!("{}: 1", i18n::tr!(HistoryMarked))),
                    "{text}"
                );
            }
        }
    }
    s.shutdown();
}

#[test]
fn shift_arrows_extend_shrink_and_start_separate_ranges_with_search_text_present() {
    let f = support::Fixture::create();
    for index in 0..7 {
        f.commit("steps.txt", &format!("step {index}\n"));
    }
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo log");
    wait(&mut s);
    command(&mut s, "repo log select");
    let ids = match &s.mode {
        Mode::History(view) => view
            .session()
            .records
            .iter()
            .map(|record| record.entry.oid.to_string())
            .collect::<Vec<_>>(),
        _ => panic!("history"),
    };
    select(&mut s, &ids[4]);
    key(&mut s, KeyCode::Char(' '));
    select(&mut s, &ids[2]);
    for code in [KeyCode::Down, KeyCode::Down, KeyCode::Up] {
        s.key(KeyEvent::new(code, KeyModifiers::SHIFT), &Cancel::default())
            .unwrap();
    }
    assert_eq!(marked(&s), [ids[4].clone(), ids[3].clone(), ids[2].clone()]);
    key(&mut s, KeyCode::Down);
    s.key(
        KeyEvent::new(KeyCode::Down, KeyModifiers::SHIFT),
        &Cancel::default(),
    )
    .unwrap();
    assert_eq!(
        marked(&s),
        [
            ids[5].clone(),
            ids[4].clone(),
            ids[3].clone(),
            ids[2].clone()
        ]
    );
    s.paste("steps");
    s.key(
        KeyEvent::new(KeyCode::Up, KeyModifiers::SHIFT),
        &Cancel::default(),
    )
    .unwrap();
    assert_eq!(s.bar.editor().text, "steps");
    assert!(matches!(&s.mode, Mode::History(view) if view.session().state.selected() == Some(4)));
    assert_eq!(marked(&s).len(), 4);
    s.shutdown();
}

#[test]
fn selection_space_marks_rows_while_nonempty_search_keeps_spaces_for_editing() {
    let f = support::Fixture::create();
    f.commit("next.txt", "next\n");
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo log");
    wait(&mut s);
    command(&mut s, "repo log select");
    key(&mut s, KeyCode::Char(' '));
    assert_eq!(marked(&s).len(), 1);
    assert!(s.bar.editor().text.is_empty());
    key(&mut s, KeyCode::Down);
    key(&mut s, KeyCode::Char(' '));
    assert_eq!(marked(&s).len(), 2);
    key(&mut s, KeyCode::Up);
    key(&mut s, KeyCode::Char(' '));
    assert_eq!(marked(&s).len(), 1);
    s.paste("initial");
    let before = marked(&s);
    key(&mut s, KeyCode::Char(' '));
    assert_eq!(s.bar.editor().text, "initial ");
    assert_eq!(marked(&s), before);
    key(&mut s, KeyCode::Esc);
    assert!(s.bar.editor().text.is_empty());
    key(&mut s, KeyCode::Char(' '));
    assert!(marked(&s).is_empty());
    assert!(render(&mut s, 120, 30).contains("[ ]"));
    s.shutdown();
}

#[test]
fn checkbox_commands_search_and_preview_return_share_a_frozen_oldest_first_selection() {
    let f = support::Fixture::create();
    support::git(&f.repo, &["checkout", "-b", "source"]);
    f.commit("picked-one.txt", "one\n");
    let first = support::git(&f.repo, &["rev-parse", "HEAD"]);
    f.commit("picked-two.txt", "two\n");
    let second = support::git(&f.repo, &["rev-parse", "HEAD"]);
    support::git(&f.repo, &["checkout", "main"]);
    f.commit("target.txt", "target\n");
    let before = support::git(&f.repo, &["rev-parse", "HEAD"]);
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo log");
    wait(&mut s);
    select(&mut s, &second);
    command(&mut s, "repo log select");
    key(&mut s, KeyCode::Insert);
    assert_eq!(marked(&s).as_slice(), std::slice::from_ref(&second));
    select(&mut s, &first);
    render(&mut s, 120, 30);
    let Mode::History(view) = &s.mode else {
        panic!("history");
    };
    let index = view.session().state.selected().unwrap();
    let point = (0..30).flat_map(|y| (0..120).map(move |x| ratatui::layout::Position::new(x,y)))
        .find(|point| matches!(s.pointer.hit(*point), Some(crate::pointer::Hit::Button(crate::pointer::Action::ToggleCommit(row))) if row == index)).unwrap();
    s.mouse(
        crossterm::event::MouseEvent {
            kind: crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left),
            column: point.x,
            row: point.y,
            modifiers: KeyModifiers::NONE,
        },
        &Cancel::default(),
    )
    .unwrap();
    assert_eq!(marked(&s), [first.clone(), second.clone()]);
    key(&mut s, KeyCode::Enter);
    assert!(s.history_actions().is_some());
    assert_eq!(support::git(&f.repo, &["rev-parse", "HEAD"]), before);
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    assert_eq!(
        batch(&s)
            .steps()
            .map(|step| step.candidate.commit.to_string())
            .collect::<Vec<_>>(),
        [first.clone(), second.clone()]
    );
    assert!(render(&mut s, 120, 30).contains(i18n::tr!(HistorySelectionBack)));
    key(&mut s, KeyCode::Esc);
    assert!(s.bar.is_plain());
    assert_eq!(marked(&s), [first.clone(), second.clone()]);
    // 退出选择会清空勾选；已有搜索文字时通过 Insert 重新勾选，搜索仍用于定位。
    s.paste("picked");
    assert_eq!(s.bar.editor().text, "picked");
    command(&mut s, "repo log select");
    assert!(marked(&s).is_empty());
    command(&mut s, "repo log select");
    select(&mut s, &first);
    key(&mut s, KeyCode::Insert);
    select(&mut s, &second);
    key(&mut s, KeyCode::Insert);
    assert_eq!(marked(&s), [first.clone(), second.clone()]);
    key(&mut s, KeyCode::Esc);
    s.paste("//repo log cherry-pick");
    select(&mut s, &before);
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    assert_eq!(
        batch(&s)
            .steps()
            .map(|step| step.candidate.commit.to_string())
            .collect::<Vec<_>>(),
        [first.clone(), second.clone()]
    );
    assert_eq!(batch(&s).first.candidate.branch, "main");
    for language in [i18n::Language::Chinese, i18n::Language::English] {
        let _language = i18n::using(language);
        let text = render(&mut s, 110, 28);
        assert!(text.contains(i18n::tr!(ReplayOrder)), "{text}");
        assert!(text.find(&first).unwrap() < text.find(&second).unwrap());
    }
    assert!(!f.repo.join("picked-one.txt").exists());
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    assert!(!matches!(s.mode, Mode::Error { .. }), "{:?}", s.mode);
    assert_eq!(support::git(&f.repo, &["rev-parse", "HEAD~2"]), before);
    assert!(f.repo.join("picked-one.txt").exists());
    assert!(f.repo.join("picked-two.txt").exists());
    assert!(marked(&s).is_empty());
    s.shutdown();
}

#[test]
fn a_batch_conflict_opens_files_and_clicking_skip_runs_the_remaining_commits() {
    let f = support::Fixture::create();
    f.commit("conflict.txt", "base\n");
    support::git(&f.repo, &["checkout", "-b", "source"]);
    let mut commits = Vec::new();
    for (path, text) in [
        ("first.txt", "first\n"),
        ("conflict.txt", "source\n"),
        ("last.txt", "last\n"),
    ] {
        f.commit(path, text);
        commits.push(support::git(&f.repo, &["rev-parse", "HEAD"]));
    }
    support::git(&f.repo, &["checkout", "main"]);
    f.commit("conflict.txt", "ours\n");
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo log");
    wait(&mut s);
    command(&mut s, "repo log select");
    for oid in &commits {
        select(&mut s, oid);
        key(&mut s, KeyCode::Insert);
    }
    command(&mut s, "repo log cherry-pick");
    wait(&mut s);
    batch(&s);
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    assert!(matches!(s.mode, Mode::Changes(_)), "{:?}", s.mode);
    let Some(State::Active(active)) = s.replay_state(&f.repo) else {
        panic!("active sequence");
    };
    assert_eq!(active.commit.to_string(), commits[1]);
    assert_eq!(active.remaining().len(), 1);
    let failed = s
        .service
        .store
        .tasks(200)
        .unwrap()
        .into_iter()
        .find(|record| record.pending() && record.spec.operation.is_some())
        .unwrap()
        .id;
    let text = render(&mut s, 140, 38);
    assert!(text.contains("conflict.txt"), "{text}");
    assert!(text.contains(i18n::tr!(ReplayConflictsShort)), "{text}");
    assert!(text.contains(i18n::tr!(ReplayRemainingShort)), "{text}");
    assert!(
        !(0..38)
            .flat_map(|y| (0..140).map(move |x| ratatui::layout::Position::new(x, y)))
            .any(|point| matches!(
                s.pointer.hit(point),
                Some(crate::pointer::Hit::Button(
                    crate::pointer::Action::ReplayControl(replay::Control::Continue)
                ))
            ))
    );
    key(&mut s, KeyCode::BackTab);
    if s.focused_conflict_control().is_some() {
        key(&mut s, KeyCode::BackTab);
    }
    render(&mut s, 140, 38);
    let point = (0..38)
        .flat_map(|y| (0..140).map(move |x| ratatui::layout::Position::new(x, y)))
        .find(|point| {
            matches!(
                s.pointer.hit(*point),
                Some(crate::pointer::Hit::Button(
                    crate::pointer::Action::ReplayControl(replay::Control::Skip)
                ))
            )
        })
        .unwrap();
    s.mouse(
        crossterm::event::MouseEvent {
            kind: crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left),
            column: point.x,
            row: point.y,
            modifiers: KeyModifiers::NONE,
        },
        &Cancel::default(),
    )
    .unwrap();
    wait(&mut s);
    assert!(
        matches!(&s.mode, Mode::Confirm { operation: Confirmation::Mutation { plan, .. }, .. } if matches!(plan.as_ref(), mutation::Plan::Replay(replay::Plan::Control(resume)) if resume.control == replay::Control::Skip))
    );
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    assert_eq!(s.replay_state(&f.repo), Some(&State::Idle));
    assert!(f.repo.join("first.txt").exists());
    assert!(f.repo.join("last.txt").exists());
    assert!(
        !s.service
            .store
            .tasks(200)
            .unwrap()
            .iter()
            .find(|record| record.id == failed)
            .unwrap()
            .pending()
    );
    s.shutdown();
}

#[test]
fn selected_commits_and_search_survive_loading_the_next_history_page() {
    let f = support::Fixture::create();
    for index in 0..105 {
        f.commit("pages.txt", &format!("page {index}\n"));
    }
    let (_home, s) = fixture(vec![f.repo.clone()]);
    let page = s
        .service
        .history(
            &f.repo,
            engine::local::history::Request::Start,
            &Cancel::default(),
        )
        .unwrap();
    let mut session = crate::history::Session::new(page, &Cancel::default()).unwrap();
    session
        .mark(crate::history::SelectionAction::ToggleCommit)
        .unwrap();
    let first = session.selected().unwrap().oid;
    session.search("pages".into());
    let query = session.query.clone();
    session.state.select(Some(session.records.len() - 2));
    session.extend_selection(1).unwrap();
    let before_page = session.picks();
    assert!(before_page.contains(&first));
    let next = s
        .service
        .history(
            &f.repo,
            engine::local::history::Request::Continue(session.cursor().unwrap()),
            &Cancel::default(),
        )
        .unwrap();
    session.append(next, &Cancel::default()).unwrap();
    assert_eq!(session.picks(), before_page);
    assert_eq!(session.query, query);
    assert!(session.records.len() > 100);
    session.extend_selection(1).unwrap();
    let next_page = session.records[100].entry.oid;
    assert!(session.picks().contains(&next_page));
    session.extend_selection(-1).unwrap();
    assert_eq!(session.picks(), before_page);
    session.state.select(Some(session.records.len() - 1));
    let oldest = session.selected().unwrap().oid;
    session
        .mark(crate::history::SelectionAction::ToggleCommit)
        .unwrap();
    let expected = std::iter::once(oldest)
        .chain(before_page)
        .collect::<Vec<_>>();
    assert_eq!(session.picks(), expected);
}

#[test]
fn an_empty_commit_pause_keeps_the_recovery_actions_in_the_changes_view() {
    let f = support::Fixture::create();
    support::git(&f.repo, &["checkout", "-b", "source"]);
    f.commit("already.txt", "same\n");
    let first = support::git(&f.repo, &["rev-parse", "HEAD"]);
    f.commit("needed.txt", "new\n");
    let second = support::git(&f.repo, &["rev-parse", "HEAD"]);
    support::git(&f.repo, &["checkout", "main"]);
    f.commit("target.txt", "target\n");
    f.commit("already.txt", "same\n");
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo log");
    wait(&mut s);
    command(&mut s, "repo log select");
    for oid in [first, second] {
        select(&mut s, &oid);
        key(&mut s, KeyCode::Insert);
    }
    command(&mut s, "repo log cherry-pick");
    wait(&mut s);
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    assert!(matches!(s.mode, Mode::Changes(_)), "{:?}", s.mode);
    assert!(
        matches!(s.replay_state(&f.repo), Some(State::Active(active)) if active.conflicts.is_empty() && active.remaining().len() == 1)
    );
    assert_eq!(s.input_page(), crate::input_pages::Page::Replay);
    let text = render(&mut s, 120, 30);
    assert!(text.contains(i18n::tr!(ReplaySkipShort)), "{text}");
    command(&mut s, "repo skip");
    wait(&mut s);
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    assert_eq!(s.replay_state(&f.repo), Some(&State::Idle));
    assert!(f.repo.join("needed.txt").exists());
    s.shutdown();
}
