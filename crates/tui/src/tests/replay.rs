use super::*;
use engine::operations::{
    mutation,
    replay::{Control, Kind, Mainline, Plan, State},
};

fn select(s: &mut Screen, oid: &str) {
    let Mode::History(view) = &mut s.mode else {
        panic!("history expected: {:?}", s.mode)
    };
    let crate::history::View::Graph(graph) = view.as_mut() else {
        panic!("graph expected")
    };
    let index = graph
        .records
        .iter()
        .position(|r| r.entry.oid.to_string() == oid)
        .unwrap();
    graph.state.select(Some(index));
}
fn reviewed(s: &Screen) -> &Plan {
    let Mode::Confirm {
        operation: Confirmation::Mutation { plan, .. },
        ..
    } = &s.mode
    else {
        panic!("review expected: {:?}", s.mode)
    };
    let mutation::Plan::Replay(plan) = plan.as_ref() else {
        panic!("replay expected")
    };
    plan
}
fn accept(s: &mut Screen) {
    reviewed(s);
    key(s, KeyCode::Enter);
    wait(s);
}
fn available(s: &mut Screen, name: &str) -> bool {
    s.paste(&format!("//{name}"));
    let found = s.search_commands().unwrap().iter().any(|c| c.name == name);
    key(s, KeyCode::Esc);
    found
}
#[test]
fn replay_uses_the_frozen_history_selection_and_only_writes_after_confirmation() {
    let f = support::Fixture::create();
    support::git(&f.repo, &["checkout", "-b", "source"]);
    f.commit("picked.txt", "picked\n");
    let source = support::git(&f.repo, &["rev-parse", "HEAD"]);
    support::git(&f.repo, &["checkout", "main"]);
    f.commit("target.txt", "target\n");
    let before = support::git(&f.repo, &["rev-parse", "HEAD"]);
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    assert!(!available(&mut s, "repo log cherry-pick"));
    command(&mut s, "repo log");
    wait(&mut s);
    select(&mut s, &source);
    s.paste("//repo log cherry-pick");
    select(&mut s, &before);
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    assert_eq!(reviewed(&s).commit().to_string(), source);
    assert_eq!(support::git(&f.repo, &["rev-parse", "HEAD"]), before);
    assert!(!f.repo.join("picked.txt").exists());
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let text = render(&mut s, 100, 24);
        assert!(text.contains("picked.txt"), "{text}");
        assert!(text.contains(i18n::tr!(ReplayTargetBranch)), "{text}");
    }
    accept(&mut s);
    assert!(!matches!(s.mode, Mode::Error { .. }), "{:?}", s.mode);
    assert_eq!(
        support::git(&f.repo, &["show", "HEAD:picked.txt"]),
        "picked"
    );
    let picked = support::git(&f.repo, &["rev-parse", "HEAD"]);
    select(&mut s, &picked);
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    // 详情中复制的仍然是文件路径，提交操作另有冻结的提交身份。
    assert_eq!(s.copy_selection().commit().unwrap().to_string(), picked);
    assert_eq!(
        s.copy_selection()
            .payload(crate::clipboard::Scope::Log, Some(&f.repo))
            .unwrap()
            .text,
        "picked.txt"
    );
    command(&mut s, "repo log revert");
    wait(&mut s);
    assert_eq!(reviewed(&s).kind(), Kind::Revert);
    accept(&mut s);
    assert!(!f.repo.join("picked.txt").exists());
    assert!(f.repo.join("target.txt").exists());
    assert!(!available(&mut s, "repo continue"));
    s.shutdown();
}
#[test]
fn merge_mainline_is_a_separate_explicit_choice_before_review() {
    let f = support::Fixture::create();
    support::git(&f.repo, &["checkout", "-b", "left"]);
    f.commit("left.txt", "left\n");
    support::git(&f.repo, &["checkout", "-b", "right", "main"]);
    f.commit("right.txt", "right\n");
    support::git(&f.repo, &["checkout", "left"]);
    support::git(
        &f.repo,
        &["merge", "--no-ff", "-m", "merge source", "right"],
    );
    let source = support::git(&f.repo, &["rev-parse", "HEAD"]);
    support::git(&f.repo, &["checkout", "main"]);
    let before = support::git(&f.repo, &["rev-parse", "HEAD"]);
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo log");
    wait(&mut s);
    select(&mut s, &source);
    command(&mut s, "repo log cherry-pick");
    wait(&mut s);
    assert!(matches!(
        &s.mode,
        Mode::Confirm {
            operation: Confirmation::ReplayParents { selected: 0, .. },
            ..
        }
    ));
    for (width, height) in [(40, 16), (90, 28), (160, 32)] {
        let text = render(&mut s, width, height);
        assert!(text.contains(i18n::tr!(ReplayParentsTitle)), "{text}");
    }
    render(&mut s, 100, 28);
    let point = (0..28)
        .flat_map(|y| (0..100).map(move |x| ratatui::layout::Position::new(x, y)))
        .find(|point| {
            matches!(
                s.pointer.hit(*point),
                Some(crate::pointer::Hit::Item(
                    crate::pointer::Target::ReplayParent,
                    1
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
    assert!(matches!(
        &s.mode,
        Mode::Confirm {
            operation: Confirmation::ReplayParents { selected: 1, .. },
            ..
        }
    ));
    key(&mut s, KeyCode::Up);
    key(&mut s, KeyCode::Down);
    let text = render(&mut s, 100, 28);
    assert!(text.contains("right.txt"), "{text}");
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    assert!(
        matches!(reviewed(&s), Plan::Start(p) if matches!(&p.mainline, Mainline::Merge(parent) if parent.number.get() == 2))
    );
    assert_eq!(support::git(&f.repo, &["rev-parse", "HEAD"]), before);
    accept(&mut s);
    assert!(f.repo.join("left.txt").exists());
    assert!(!f.repo.join("right.txt").exists());
    s.shutdown();
}
#[test]
fn conflicts_survive_restart_and_resolving_via_worktree_clears_the_pending_task() {
    let f = support::Fixture::create();
    f.commit("conflict.txt", "base\n");
    support::git(&f.repo, &["checkout", "-b", "source"]);
    f.commit("conflict.txt", "source\n");
    let source = support::git(&f.repo, &["rev-parse", "HEAD"]);
    support::git(&f.repo, &["checkout", "main"]);
    f.commit("conflict.txt", "ours\n");
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo log");
    wait(&mut s);
    select(&mut s, &source);
    command(&mut s, "repo log cherry-pick");
    wait(&mut s);
    accept(&mut s);
    assert!(matches!(s.mode, Mode::Changes(_)), "{:?}", s.mode);
    let failed = s
        .service
        .store
        .tasks(200)
        .unwrap()
        .into_iter()
        .find(|r| r.pending() && r.spec.operation.is_some())
        .unwrap()
        .id;
    let service = Service::new(s.service.store.clone(), s.service.scheduler.clone());
    s.shutdown();
    let mut s = screen(service);
    command(&mut s, "repo diff");
    wait(&mut s);
    assert!(matches!(s.replay_state(&f.repo), Some(State::Active(_))));
    assert!(available(&mut s, "repo continue"));
    assert!(available(&mut s, "repo skip"));
    assert!(available(&mut s, "repo abort"));
    assert!(!available(&mut s, "repo log cherry-pick"));
    let text = render(&mut s, 140, 38);
    assert!(text.contains(i18n::tr!(ReplayContinueShort)), "{text}");
    assert!(text.contains(i18n::tr!(ReplayConflictWorktree)), "{text}");
    assert!(text.contains("<<<<<<<"), "{text}");
    support::write(f.repo.join("conflict.txt"), "resolved\n");
    command(&mut s, "repo diff stage");
    wait(&mut s);
    assert!(!matches!(s.mode, Mode::Error { .. }), "{:?}", s.mode);
    assert!(support::git(&f.repo, &["ls-files", "-u"]).is_empty());
    command(&mut s, "repo continue");
    wait(&mut s);
    assert!(matches!(reviewed(&s), Plan::Control(p) if p.control == Control::Continue));
    accept(&mut s);
    assert_eq!(
        support::git(&f.repo, &["show", "HEAD:conflict.txt"]),
        "resolved"
    );
    assert_eq!(s.replay_state(&f.repo), Some(&State::Idle));
    assert!(
        !s.service
            .store
            .tasks(200)
            .unwrap()
            .iter()
            .find(|r| r.id == failed)
            .unwrap()
            .pending()
    );
    assert!(!available(&mut s, "repo continue"));
    s.shutdown();
}
