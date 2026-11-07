use super::breadcrumbs::{click_crumb, labels};
use super::*;
use crate::navigation::breadcrumbs::{Destination, Level};

#[test]
fn management_settings_and_editors_follow_their_displayed_parents() {
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let (_home, mut s) = fixture(vec![]);
        s.config.ai.enabled = true;
        let config = std::fs::read(s.service.store.config_path()).unwrap();
        for name in [
            "repos manage",
            "settings",
            "schedule",
            "schedule actions",
            "settings ai prompt",
        ] {
            command(&mut s, name);
            let trail = s.breadcrumbs();
            let parent = trail[trail.len() - 2].destination.unwrap();
            let depth = trail.len();
            click_crumb(&mut s, parent);
            assert_eq!(labels(&s).len(), depth - 1, "{name}");
            assert!(matches!(s.mode, Mode::Browsing), "{name}: {:?}", s.mode);
            if s.page != Page::Repositories {
                click_crumb(&mut s, Destination::Home);
            }
            assert!(s.history.is_empty());
            assert_eq!(s.page, Page::Repositories);
            assert_eq!(
                std::fs::read(s.service.store.config_path()).unwrap(),
                config
            );
        }
        s.shutdown();
    }
}

#[test]
fn scan_results_and_error_parents_return_to_the_actual_management_session() {
    let f = support::Fixture::create();
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repos manage");
    let origin = Box::new(s.take_mode());
    s.mode = Mode::Error {
        origin,
        title: "读取失败".into(),
        detail: "测试错误".into(),
        scroll: 0,
    };
    click_crumb(&mut s, Destination::Current(Level::PageRoot));
    assert!(
        matches!(&s.mode, Mode::Management(session) if session.registered_path() == Some(f.repo.as_path()))
    );
    command(
        &mut s,
        &format!("repos scan \"{}\"", f.temp.path().display()),
    );
    wait(&mut s);
    assert!(
        matches!(&s.mode, Mode::Management(session) if matches!(session.source, crate::management::Source::Scan { .. }))
    );
    let parent = s.breadcrumbs()[1].destination.unwrap();
    click_crumb(&mut s, parent);
    assert!(
        matches!(&s.mode, Mode::Management(session) if matches!(session.source, crate::management::Source::Managed))
    );
    click_crumb(&mut s, Destination::Home);
    assert_eq!(s.page, Page::Repositories);
    assert!(matches!(s.mode, Mode::Browsing));
    s.shutdown();
}

#[test]
fn prompt_error_distinguishes_the_editor_from_its_settings_parent() {
    let (_home, mut s) = fixture(vec![]);
    s.config.ai.enabled = true;
    command(&mut s, "settings ai prompt");
    let origin = Box::new(s.take_mode());
    s.mode = Mode::Error {
        origin,
        title: "保存失败".into(),
        detail: "测试错误".into(),
        scroll: 0,
    };
    click_crumb(&mut s, Destination::Current(Level::Page));
    assert!(matches!(s.mode, Mode::PromptEditor(_)));
    click_crumb(&mut s, Destination::Current(Level::PageRoot));
    assert_eq!(s.page, Page::Configuration(Configuration::General));
    assert!(matches!(s.mode, Mode::Browsing));
    click_crumb(&mut s, Destination::Home);
    assert!(s.history.is_empty());
    s.shutdown();
}

#[test]
fn confirmation_parents_cancel_the_dialog_and_preserve_the_selected_repository() {
    let (_home, mut s) = fixture(vec!["/example/first".into(), "/example/second".into()]);
    let before = s.service.store.config().unwrap().repositories;
    command(&mut s, "repos manage");
    key(&mut s, KeyCode::Down);
    command(&mut s, "repos remove");
    assert!(matches!(s.mode, Mode::Confirm { .. }));
    let origin = Box::new(s.take_mode());
    s.mode = Mode::Error {
        origin,
        title: "读取失败".into(),
        detail: "测试错误".into(),
        scroll: 0,
    };
    click_crumb(&mut s, Destination::Current(Level::Page));
    assert!(matches!(s.mode, Mode::Confirm { .. }));
    click_crumb(&mut s, Destination::Current(Level::PageRoot));
    assert!(
        matches!(&s.mode, Mode::Management(session) if session.registered_path() == Some(before[1].as_path()))
    );
    command(&mut s, "repos remove");
    click_crumb(&mut s, Destination::Home);
    assert_eq!(s.page, Page::Repositories);
    assert!(matches!(s.mode, Mode::Browsing));
    command(&mut s, "repos remove");
    click_crumb(&mut s, Destination::Home);
    assert!(matches!(s.mode, Mode::Browsing));
    assert!(s.history.is_empty());
    assert_eq!(s.service.store.config().unwrap().repositories, before);
    s.shutdown();
}

#[test]
fn commit_forms_return_home_and_to_the_diff_root_without_committing() {
    let f = support::Fixture::create();
    support::write(f.repo.join("staged.txt"), "staged content\n");
    support::git(&f.repo, &["add", "staged.txt"]);
    let head = support::git(&f.repo, &["rev-parse", "HEAD"]);
    let staged = support::git(&f.repo, &["diff", "--cached"]);
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo commit");
    wait(&mut s);
    assert!(matches!(s.mode, Mode::CommitMessage { .. }));
    click_crumb(&mut s, Destination::Home);
    assert!(matches!(s.mode, Mode::Browsing));
    assert_eq!(s.page, Page::Repositories);
    command(&mut s, "repo diff");
    wait(&mut s);
    command(&mut s, "repo commit");
    wait(&mut s);
    click_crumb(&mut s, Destination::Current(Level::PageRoot));
    wait(&mut s);
    assert_eq!(s.page, Page::View(crate::navigation::View::Diff));
    assert!(matches!(s.mode, Mode::Changes(_)));
    assert_eq!(support::git(&f.repo, &["rev-parse", "HEAD"]), head);
    assert_eq!(support::git(&f.repo, &["diff", "--cached"]), staged);
    s.shutdown();
}

#[test]
fn push_confirmation_and_target_return_home_without_publishing() {
    for force in [false, true] {
        let f = support::Fixture::create();
        if force {
            f.advance();
            f.commit("local.txt", "local\n");
        } else {
            support::git(&f.repo, &["branch", "--unset-upstream"]);
            support::git(&f.repo, &["config", "push.autoSetupRemote", "false"]);
        }
        let remote = support::git(&f.remote, &["rev-parse", "main"]);
        let (_home, mut s) = fixture(vec![f.repo.clone()]);
        command(&mut s, "repo push");
        wait(&mut s);
        assert!(if force {
            matches!(s.mode, Mode::PushConfirm { .. })
        } else {
            matches!(s.mode, Mode::PushTarget(_))
        });
        click_crumb(&mut s, Destination::Home);
        assert_eq!(s.page, Page::Repositories);
        assert!(matches!(s.mode, Mode::Browsing));
        assert_eq!(support::git(&f.remote, &["rev-parse", "main"]), remote);
        s.shutdown();
    }
}

#[test]
fn blocking_work_does_not_advertise_inactive_breadcrumb_links() {
    let (_home, mut s) = fixture(vec![]);
    command(&mut s, "repos manage");
    let (release, gate) = std::sync::mpsc::channel();
    s.start("测试读取", Effect::ReadOnly, move |_, _, _| {
        gate.recv_timeout(Duration::from_secs(3))?;
        Ok(Outcome::Text("完成".into()))
    });
    render(&mut s, 120, 30);
    assert!(s.breadcrumb_items.is_empty());
    s.follow_breadcrumb(Destination::Home);
    assert!(matches!(s.mode, Mode::Busy(_)));
    release.send(()).unwrap();
    wait(&mut s);
    click_crumb(&mut s, Destination::Home);
    assert_eq!(s.page, Page::Repositories);
    assert!(matches!(s.mode, Mode::Browsing));
    s.shutdown();
}

#[test]
fn leaving_generation_cancels_it_and_late_output_cannot_reopen_the_form() {
    fn response(
        cancel: &Cancel,
        _: &mut dyn FnMut(engine::commit_message::Update),
    ) -> anyhow::Result<String> {
        let start = Instant::now();
        while start.elapsed() < Duration::from_secs(3) {
            cancel.check()?;
            thread::sleep(Duration::from_millis(3));
        }
        Ok("late message".into())
    }
    let f = support::Fixture::create();
    support::write(f.repo.join("staged.txt"), "content\n");
    support::git(&f.repo, &["add", "staged.txt"]);
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    s.config.ai.enabled = true;
    s.commit_generator.response = Some(response);
    command(&mut s, "repo commit");
    wait(&mut s);
    s.generate_commit_message();
    assert!(s.commit_generator.running());
    click_crumb(&mut s, Destination::Home);
    let start = Instant::now();
    while s.commit_generator.running() {
        assert!(start.elapsed() < Duration::from_secs(1));
        s.poll_commit_message();
        thread::sleep(Duration::from_millis(3));
    }
    assert_eq!(s.page, Page::Repositories);
    assert!(matches!(s.mode, Mode::Browsing));
    s.shutdown();
}
