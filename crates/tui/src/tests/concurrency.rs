use super::*;

#[test]
fn slow_commit_keeps_navigation_commands_and_other_repository_operations_available() {
    let _language = i18n::using(i18n::Language::English);
    let first = support::Fixture::create();
    let second = support::Fixture::create();
    support::write(first.repo.join("first"), "first");
    support::git(&first.repo, &["add", "first"]);
    support::write(second.repo.join("second"), "second");
    let (_home, mut s) = fixture(vec![first.repo.clone(), second.repo.clone()]);
    s.focus(Some(first.repo.clone()));
    let plan = s
        .service
        .prepare_commit(&first.repo, &Cancel::default())
        .unwrap();
    let (release, gate) = mpsc::channel();
    s.start_repository(
        first.repo.clone(),
        "delayed commit",
        Effect::Commit,
        move |service, cancel, _| {
            gate.recv_timeout(Duration::from_secs(8)).unwrap();
            Ok(Outcome::Committed(service.commit(&plan, "first", &cancel)?))
        },
    );
    assert!(matches!(s.mode, Mode::Browsing));
    assert!(render(&mut s, 140, 30).contains("delayed commit"));
    command(&mut s, "sort name");
    command(&mut s, "filter all");
    assert_eq!(s.order, Order::Name);
    assert_eq!(s.selected_path(), Some(first.repo.clone()));
    assert!(s.repository_tasks.contains_key(&first.repo));
    command(&mut s, "help");
    assert_eq!(s.page, Page::Help);
    key(&mut s, KeyCode::Esc);
    s.focus(Some(second.repo.clone()));
    open_files(&mut s);
    poll_background_until(&mut s, |s| !s.repository_tasks.contains_key(&second.repo));
    assert!(matches!(&s.mode, Mode::Files(session) if session.snapshot.path == second.repo));
    command(&mut s, "diff stage");
    poll_background_until(&mut s, |s| !s.repository_tasks.contains_key(&second.repo));
    assert_eq!(
        support::git(&second.repo, &["diff", "--cached", "--name-only"]),
        "second"
    );
    assert!(s.repository_tasks.contains_key(&first.repo));
    release.send(()).unwrap();
    wait(&mut s);
    assert!(matches!(&s.mode, Mode::Files(session) if session.snapshot.path == second.repo));
    assert!(matches!(
        s.result_for(&first.repo),
        Some(RepositoryResult::Attention { .. })
    ));
    assert!(matches!(
        s.result_for(&second.repo),
        Some(RepositoryResult::Attention { .. })
    ));
    let reopened = Screen::new((*s.service).clone(), &Cancel::default()).unwrap();
    assert!(matches!(
        reopened.result_for(&first.repo),
        Some(RepositoryResult::Attention { .. })
    ));
    assert!(matches!(
        reopened.result_for(&second.repo),
        Some(RepositoryResult::Attention { .. })
    ));
}
#[test]
fn late_batch_completion_cannot_replace_a_newer_repository_operation_even_if_its_clock_is_older() {
    let first = support::Fixture::create();
    let second = support::Fixture::create();
    let (_home, mut s) = fixture(vec![first.repo.clone(), second.repo.clone()]);
    let gate = FetchGate::install(&second);
    command(&mut s, "check all");
    poll_background_until(&mut s, |s| {
        gate.started.exists()
            && matches!(
                s.result_for(&first.repo),
                Some(RepositoryResult::Clean { .. })
            )
    });
    support::write(first.repo.join("new"), "new");
    support::git(&first.repo, &["add", "new"]);
    let plan = s
        .service
        .prepare_commit(&first.repo, &Cancel::default())
        .unwrap();
    s.focus(Some(first.repo.clone()));
    s.start_repository(
        first.repo.clone(),
        "commit",
        Effect::Commit,
        move |service, cancel, _| Ok(Outcome::Committed(service.commit(&plan, "new", &cancel)?)),
    );
    poll_background_until(&mut s, |s| s.repository_tasks.is_empty());
    assert!(matches!(
        s.result_for(&first.repo),
        Some(RepositoryResult::Attention { .. })
    ));
    let revision = s.latest_update(&first.repo).unwrap().revision;
    gate.release();
    poll_background_until(&mut s, |s| s.check_task.is_none());
    assert_eq!(s.latest_update(&first.repo).unwrap().revision, revision);
    assert!(matches!(
        s.result_for(&first.repo),
        Some(RepositoryResult::Attention { .. })
    ));
    let mut report = s
        .service
        .store
        .repository_report(&first.repo)
        .unwrap()
        .unwrap();
    report.finished_at = "2000-01-01T00:00:00+00:00".into();
    s.service
        .store
        .save_repository_report(&mut report, &Cancel::default())
        .unwrap();
    let reopened = Screen::new((*s.service).clone(), &Cancel::default()).unwrap();
    assert!(matches!(
        reopened.result_for(&first.repo),
        Some(RepositoryResult::Attention { .. })
    ));
}
#[test]
fn two_single_repository_checks_overlap_and_same_repository_commands_do_not_start_duplicates() {
    let first = support::Fixture::create();
    let second = support::Fixture::create();
    let gate_a = FetchGate::install(&first);
    let gate_b = FetchGate::install(&second);
    let (_home, mut s) = fixture(vec![first.repo.clone(), second.repo.clone()]);
    s.focus(Some(first.repo.clone()));
    command(&mut s, "check");
    s.focus(Some(second.repo.clone()));
    command(&mut s, "check");
    poll_background_until(&mut s, |_| {
        gate_a.started.exists() && gate_b.started.exists()
    });
    assert_eq!(s.repository_tasks.len(), 2);
    command(&mut s, "commit");
    assert_eq!(s.repository_tasks.len(), 2);
    assert!(s.message.contains("already running"));
    gate_a.release();
    poll_background_until(&mut s, |s| !s.repository_tasks.contains_key(&first.repo));
    assert!(s.repository_tasks.contains_key(&second.repo));
    gate_b.release();
    wait(&mut s);
    assert!(s.service.store.report().unwrap().is_none());
    assert_eq!(s.service.store.repository_reports().unwrap().len(), 2);
}
#[test]
fn cancelled_repository_task_does_not_take_over_the_current_view() {
    let _language = i18n::using(i18n::Language::English);
    let f = support::Fixture::create();
    let gate = FetchGate::install(&f);
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "check");
    poll_background_until(&mut s, |_| gate.started.exists());
    command(&mut s, "cancel");
    wait(&mut s);
    assert!(matches!(s.mode, Mode::Browsing));
    assert!(s.message.contains("cancelled"));
    assert!(s.service.store.repository_reports().unwrap().is_empty());
}

#[test]
fn task_completion_preserves_a_new_command_draft_and_errors_belong_to_the_original_repository() {
    let f = support::Fixture::create();
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    let (release, gate) = mpsc::channel();
    s.start_repository(
        f.repo.clone(),
        "delayed read",
        Effect::ReadOnly,
        move |service, cancel, _| {
            gate.recv_timeout(Duration::from_secs(8)).unwrap();
            Ok(Outcome::Files(service.files(&f.repo, &cancel)?))
        },
    );
    s.paste("//settings con");
    release.send(()).unwrap();
    wait(&mut s);
    assert!(matches!(s.mode, Mode::Browsing));
    assert_eq!(s.bar.text(), "/settings con");
    key(&mut s, KeyCode::Esc);
    let path = s.selected_path().unwrap();
    let (release, gate) = mpsc::channel();
    s.start_repository(
        path.clone(),
        "failed read",
        Effect::ReadOnly,
        move |_, _, _| {
            gate.recv_timeout(Duration::from_secs(8)).unwrap();
            anyhow::bail!("repository-specific failure")
        },
    );
    command(&mut s, "help");
    release.send(()).unwrap();
    wait(&mut s);
    assert_eq!(s.page, Page::Help);
    assert!(matches!(s.mode, Mode::Browsing));
    assert!(s.repository_errors[&path].contains("repository-specific failure"));
}
