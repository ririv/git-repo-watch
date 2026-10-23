use super::*;

fn accept(s: &mut Screen) {
    assert!(
        matches!(
            s.mode,
            Mode::Confirm {
                operation: Confirmation::Mutation { .. },
                ..
            }
        ),
        "{:?}",
        s.mode
    );
    key(s, KeyCode::Enter);
    assert!(!matches!(s.mode, Mode::Busy(_)));
    wait(s);
    assert!(!matches!(s.mode, Mode::Error { .. }), "{:?}", s.mode);
}

#[test]
fn branch_parameter_review_returns_exact_draft_and_switches_only_after_acceptance() {
    let f = support::Fixture::create();
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "branch create feature/中文");
    wait(&mut s);
    let draft = "/branch create feature/中文";
    assert_eq!(support::git(&f.repo, &["branch", "--show-current"]), "main");
    key(&mut s, KeyCode::Esc);
    assert_eq!(s.bar.text(), draft);
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        for width in [60, 100, 180] {
            let text = render(&mut s, width, 28);
            assert!(text.contains("feature/中文"), "{text}");
            assert!(text.contains(i18n::tr!(MutationRun)), "{text}");
        }
    }
    accept(&mut s);
    assert_eq!(support::git(&f.repo, &["branch", "--show-current"]), "main");
    command(&mut s, "branch switch feature/中文");
    wait(&mut s);
    accept(&mut s);
    assert_eq!(
        support::git(&f.repo, &["branch", "--show-current"]),
        "feature/中文"
    );
    assert!(s.updates.contains_key(&f.repo));
    s.shutdown();
}

#[test]
fn stash_context_prefills_selected_identity_and_retains_selection_after_apply() {
    let f = support::Fixture::create();
    f.commit("tracked", "base\n");
    support::write(f.repo.join("tracked"), "changed\n");
    support::write(f.repo.join("未跟踪 file"), "new\n");
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "stash save-all 保存当前工作");
    wait(&mut s);
    accept(&mut s);
    assert!(support::git(&f.repo, &["status", "--porcelain"]).is_empty());
    command(&mut s, "stash");
    wait(&mut s);
    let Mode::Stashes(view) = &s.mode else {
        panic!("{:?}", s.mode)
    };
    let oid = view.selected().unwrap().oid;
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        for width in [60, 100, 180] {
            let text = render(&mut s, width, 28);
            assert!(text.contains("stash@{0}"), "{text}");
            assert!(text.contains("保存当前工作"), "{text}");
        }
    }
    key(&mut s, KeyCode::Char('/'));
    s.paste("apply");
    key(&mut s, KeyCode::Tab);
    assert_eq!(s.bar.text(), format!("/stash apply {oid}"));
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    accept(&mut s);
    let Mode::Stashes(view) = &s.mode else {
        panic!("{:?}", s.mode)
    };
    assert_eq!(view.selected().unwrap().oid, oid);
    assert_eq!(
        std::fs::read_to_string(f.repo.join("未跟踪 file")).unwrap(),
        "new\n"
    );
    key(&mut s, KeyCode::Enter);
    assert!(
        matches!(&s.mode,Mode::Stashes(v) if matches!(v.as_ref(),crate::stashes::View::Detail{..}))
    );
    key(&mut s, KeyCode::Esc);
    key(&mut s, KeyCode::Char('/'));
    s.paste("drop");
    key(&mut s, KeyCode::Enter);
    assert_eq!(s.bar.text(), format!("/stash drop {oid}"));
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    accept(&mut s);
    assert!(matches!(&s.mode,Mode::Stashes(v) if v.session().snapshot.entries.is_empty()));
    assert_eq!(
        std::fs::read_to_string(f.repo.join("tracked")).unwrap(),
        "changed\n"
    );
    s.shutdown();
}

#[test]
fn fetch_is_immediate_and_pull_reviews_then_fast_forwards_without_pushing() {
    let f = support::Fixture::create();
    f.advance();
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    let before = support::git(&f.repo, &["rev-parse", "HEAD"]);
    command(&mut s, "fetch");
    assert!(!s.repository_tasks.is_empty());
    assert!(!matches!(s.mode, Mode::Confirm { .. } | Mode::Busy(_)));
    wait(&mut s);
    assert_eq!(support::git(&f.repo, &["rev-parse", "HEAD"]), before);
    command(&mut s, "stash");
    wait(&mut s);
    command(&mut s, "pull");
    wait(&mut s);
    assert_eq!(support::git(&f.repo, &["rev-parse", "HEAD"]), before);
    accept(&mut s);
    assert_eq!(
        support::git(&f.repo, &["rev-parse", "HEAD"]),
        support::git(&f.repo, &["rev-parse", "origin/main"])
    );
    command(&mut s, "pull");
    wait(&mut s);
    assert!(!matches!(s.mode, Mode::Confirm { .. } | Mode::Error { .. }));
    assert_eq!(s.message, i18n::tr!(PullCurrent));
    s.shutdown();
}

#[test]
fn pending_branch_review_never_overwrites_new_input_or_another_repository() {
    let f = support::Fixture::create();
    let other = support::Fixture::create();
    let (_home, mut s) = fixture(vec![f.repo.clone(), other.repo.clone()]);
    s.focus(Some(f.repo.clone()));
    command(&mut s, "branch create not-confirmed");
    s.paste("x");
    let draft = s.bar.text();
    wait(&mut s);
    assert_eq!(s.bar.text(), draft);
    assert!(!matches!(s.mode, Mode::Confirm { .. }));
    assert_eq!(
        support::git(&f.repo, &["branch", "--list", "not-confirmed"]),
        ""
    );
    key(&mut s, KeyCode::Esc);
    s.focus(Some(other.repo.clone()));
    assert!(s.updates.is_empty());
    s.shutdown();
}

#[test]
fn mutation_readback_updates_files_without_replacing_a_new_scoped_command() {
    let f = support::Fixture::create();
    f.commit("file", "base\n");
    support::write(f.repo.join("file"), "changed\n");
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "diff");
    wait(&mut s);
    let plan = s
        .service
        .prepare_mutation(
            &f.repo,
            engine::operations::mutation::Request::Stash(engine::local::stash::Request::Save {
                mode: engine::local::stash::SaveMode::Tracked,
                message: "queued".into(),
            }),
            &Cancel::default(),
        )
        .unwrap();
    let (release, gate) = mpsc::channel();
    s.start_repository(
        f.repo.clone(),
        "save",
        Effect::RepositoryWrite,
        move |service, cancel, _| {
            gate.recv_timeout(Duration::from_secs(8)).unwrap();
            Ok(Outcome::MutationCompleted(service.mutate(&plan, &cancel)?))
        },
    );
    s.paste("/layout");
    let draft = s.bar.text();
    release.send(()).unwrap();
    wait(&mut s);
    assert_eq!(s.bar.text(), draft);
    assert!(s.mode.file_session().unwrap().snapshot.changes.is_empty());
    assert!(s.updates.contains_key(&f.repo));
    s.shutdown();
}
