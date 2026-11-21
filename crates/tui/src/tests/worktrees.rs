use super::*;

#[test]
fn worktree_list_creates_opens_and_removes_a_linked_checkout() {
    let repo = support::Fixture::create();
    repo.commit("tracked", "one\n");
    let (_home, mut screen) = fixture(vec![repo.repo.clone()]);
    let target = repo.temp.path().join("独立 工作树");

    screen.open_repository(repo.repo.clone());
    command(&mut screen, "repo worktree");
    wait(&mut screen);
    assert!(
        matches!(&screen.mode, Mode::Worktrees(session) if session.snapshot.entries.len() == 1)
    );

    command(
        &mut screen,
        &format!("repo worktree add --new topic \"{}\"", target.display()),
    );
    wait(&mut screen);
    confirm_worktree(&mut screen);
    assert!(
        matches!(&screen.mode, Mode::Worktrees(session) if session.snapshot.entries.len() == 2)
    );
    let visible = render(&mut screen, 220, 25);
    assert!(visible.contains(i18n::tr!(WorktreeCurrent)));
    assert!(visible.contains("topic"));
    assert!(visible.contains("独立 工作树"));
    key(&mut screen, KeyCode::Down);
    key(&mut screen, KeyCode::Enter);
    let target = target.canonicalize().unwrap();
    assert_eq!(screen.scope_path().as_ref(), Some(&target));
    assert!(screen.service.files(&target, &Cancel::default()).is_ok());
    assert_eq!(
        screen.service.store.config().unwrap().repositories,
        vec![repo.repo.clone()]
    );

    screen.return_to_view();
    assert!(
        matches!(&screen.mode, Mode::Worktrees(session) if session.selected().is_some_and(|entry| entry.path == target))
    );
    command(&mut screen, "repo worktree remove");
    wait(&mut screen);
    assert!(matches!(
        screen.mode,
        Mode::Confirm {
            choice: crate::confirmation::Choice::Return,
            ..
        }
    ));
    assert!(render(&mut screen, 220, 25).contains("独立 工作树"));
    confirm_worktree(&mut screen);
    assert!(!target.exists());
    assert!(
        matches!(&screen.mode, Mode::Worktrees(session) if session.snapshot.entries.len() == 1)
    );
    key(&mut screen, KeyCode::Esc);
    assert_eq!(screen.page, Page::Detail);
    assert_eq!(screen.scope_path().as_ref(), Some(&repo.repo));
    screen.shutdown();
}

fn confirm_worktree(screen: &mut Screen) {
    assert!(
        matches!(&screen.mode, Mode::Confirm {
        operation: Confirmation::Mutation { plan, .. }, ..
    } if matches!(plan.as_ref(), engine::operations::mutation::Plan::Worktree(_))),
        "{:?}",
        screen.message
    );
    key(screen, KeyCode::Left);
    key(screen, KeyCode::Enter);
    wait(screen);
}
