use super::*;
use crate::loading::{FEEDBACK_DELAY, Feedback};
use std::time::Instant;

fn pending(s: &mut Screen) -> &mut Pending {
    let Mode::Busy(pending) = &mut s.mode else {
        panic!("foreground task is pending")
    };
    pending
}
fn started(s: &mut Screen) -> Instant {
    let Feedback::Quiet { since, .. } = &pending(s).feedback else {
        panic!("task starts quietly")
    };
    *since
}
fn controlled(s: &mut Screen) -> mpsc::Sender<()> {
    let (release, gate) = mpsc::channel();
    s.start("controlled task", Effect::ReadOnly, move |_, cancel, _| {
        gate.recv()?;
        cancel.check()?;
        Ok(Outcome::Text("finished".into()))
    });
    release
}
fn footer(s: &mut Screen, width: u16, height: u16) -> Vec<String> {
    let text = render(s, width, height);
    let input = s.input_area.expect("bordered input");
    text.lines()
        .skip(input.y.into())
        .take(input.height as usize + 1)
        .map(str::to_owned)
        .collect()
}

#[test]
fn fast_ai_toggle_retains_input_presentation_and_applies_without_waiting_for_feedback() {
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let (_home, mut s) = fixture(vec![]);
        command(&mut s, "settings");
        let items = Configuration::General.items(&s.snapshot, &s.config.ai);
        s.setting_selected = items
            .iter()
            .position(|item| *item == crate::settings::Item::AiEnabled)
            .unwrap();
        let before = footer(&mut s, 100, 32);
        let enabled = s.config.ai.enabled;
        s.activate_setting(crate::settings::Item::AiEnabled, &Cancel::default())
            .unwrap();
        let now = started(&mut s);
        assert_eq!(footer(&mut s, 100, 32), before);
        assert!(!pending(&mut s).tick(now + FEEDBACK_DELAY - Duration::from_millis(1)));
        assert_eq!(footer(&mut s, 100, 32), before);
        wait(&mut s);
        assert_eq!(s.config.ai.enabled, !enabled);
        assert_eq!(s.service.store.config().unwrap().ai.enabled, !enabled);
        assert!(
            !footer(&mut s, 100, 32)
                .join("\n")
                .contains(i18n::tr!(RunningAlt))
        );
        s.shutdown();
    }
}

#[test]
fn long_task_reveals_status_only_on_the_last_line_and_keeps_input_stable() {
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let (_home, mut s) = fixture(vec![fixture_path!("/example/alpha").into()]);
        s.paste("alpha");
        let before = footer(&mut s, 100, 32);
        let release = controlled(&mut s);
        let now = started(&mut s);
        assert_eq!(footer(&mut s, 100, 32), before);
        key(&mut s, KeyCode::Char('x'));
        assert_eq!(footer(&mut s, 100, 32), before);
        assert!(!pending(&mut s).tick(now + FEEDBACK_DELAY - Duration::from_millis(1)));
        assert!(pending(&mut s).tick(now + FEEDBACK_DELAY));
        let visible = render(&mut s, 100, 32);
        let last = visible.lines().last().unwrap();
        assert!(last.contains(i18n::tr!(RunningAlt)), "{visible}");
        assert!(last.contains("controlled task"), "{visible}");
        assert!(
            !visible
                .lines()
                .take(31)
                .any(|line| line.contains("controlled task"))
        );
        let after = footer(&mut s, 100, 32);
        assert_eq!(&after[..after.len() - 1], &before[..before.len() - 1]);
        release.send(()).unwrap();
        wait(&mut s);
        assert!(!matches!(s.mode, Mode::Busy(_)));
        s.shutdown();
    }
}

#[test]
fn cancellation_and_failure_are_visible_without_waiting_for_running_feedback() {
    let (_home, mut s) = fixture(vec![]);
    command(&mut s, "settings");
    let release = controlled(&mut s);
    key(&mut s, KeyCode::Esc);
    assert_eq!(pending(&mut s).state, TaskState::Cancelling);
    assert!(
        render(&mut s, 100, 32)
            .lines()
            .last()
            .unwrap()
            .contains(i18n::tr!(Cancelling))
    );
    release.send(()).unwrap();
    wait(&mut s);
    assert!(matches!(s.mode, Mode::Error { .. }));
    key(&mut s, KeyCode::Esc);
    s.start("fast failure", Effect::ReadOnly, |_, _, _| {
        anyhow::bail!("failure remains visible")
    });
    wait(&mut s);
    let text = render(&mut s, 100, 32);
    assert!(text.contains("failure remains visible"), "{text}");
    assert!(text.lines().last().unwrap().contains("fast failure"));
    s.shutdown();
}

#[test]
fn check_progress_and_cancellation_share_the_last_row_without_moving_input() {
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let f = support::Fixture::create();
        let (_home, mut s) = fixture(vec![f.repo.clone()]);
        render(&mut s, 100, 32);
        let input = s.input_area;
        let gate = gate_git(&mut s, &f, GitOperation::Fetch);
        s.start_check(crate::checks::CheckTarget::All);
        poll_background_until(&mut s, |_| gate.started.exists());
        let task = s.check_task.as_ref().unwrap();
        assert!(task.checking(&f.repo));
        let summary = task.summary().trim().to_owned();
        let spinner = task.spinner();
        let text = render(&mut s, 100, 32);
        assert!(text.lines().last().unwrap().contains(&summary), "{text}");
        assert!(!text.lines().take(31).any(|line| line.contains(&summary)));
        assert!(text.lines().take(31).any(|line| line.contains(spinner)));
        assert_eq!(s.input_area, input);
        command(&mut s, "help");
        s.paste("/");
        let text = render(&mut s, 100, 32);
        assert!(text.lines().last().unwrap().contains(&summary), "{text}");
        s.cancel_check();
        let summary = s.check_task.as_ref().unwrap().summary().trim().to_owned();
        let text = render(&mut s, 100, 32);
        let last = text.lines().last().unwrap();
        assert!(last.contains(i18n::tr!(CheckCancelRequested)), "{text}");
        assert!(!last.contains(&summary), "{text}");
        assert!(!text.lines().next().unwrap().contains(&summary));
        gate.release();
        poll_background_until(&mut s, |s| s.check_task.is_none());
        s.shutdown();
    }
}

#[test]
fn repository_progress_stays_in_the_footer_across_navigation_and_cancellation() {
    let f = support::Fixture::create();
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    let (release, gate) = mpsc::channel();
    s.start_repository(
        f.repo.clone(),
        "isolated repository operation",
        Effect::ReadOnly,
        move |_, cancel, _| {
            gate.recv()?;
            cancel.check()?;
            Ok(Outcome::Text("finished".into()))
        },
    );
    assert!(matches!(s.mode, Mode::Browsing));
    for destination in ["help", "settings", "repos"] {
        command(&mut s, destination);
        let text = render(&mut s, 100, 32);
        assert!(
            text.lines()
                .last()
                .unwrap()
                .contains("isolated repository operation"),
            "{text}"
        );
        assert!(
            !text
                .lines()
                .next()
                .unwrap()
                .contains("isolated repository operation")
        );
    }
    assert!(s.cancel_repository_task());
    let text = render(&mut s, 100, 32);
    assert!(
        text.lines().last().unwrap().contains(i18n::tr!(Cancelling)),
        "{text}"
    );
    release.send(()).unwrap();
    wait(&mut s);
    s.shutdown();
}

#[test]
fn external_progress_and_pending_failures_remain_visible_in_small_windows() {
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let (_home, mut s) = fixture(vec![]);
        s.snapshot.activity = Activity::Running;
        for (width, height) in [(100, 32), (60, 18), (24, 8)] {
            let text = render(&mut s, width, height);
            assert!(
                text.lines()
                    .last()
                    .unwrap()
                    .contains(i18n::tr!(CheckingAltAlt)),
                "{text}"
            );
            assert!(
                !text
                    .lines()
                    .next()
                    .unwrap()
                    .contains(i18n::tr!(CheckingAltAlt))
            );
        }
        let run = engine::tasks::Run::begin(
            &s.service.store,
            crate::task_center::spec("failed check", None, engine::tasks::Retry::FetchAll),
            &Cancel::default(),
        )
        .unwrap();
        run.finish(engine::tasks::Outcome::failure(
            "failure details remain in the task view",
        ))
        .unwrap();
        let journal = s.service.store.task_journal(200, None).unwrap();
        s.tasks.records = journal.records;
        s.tasks.unread_failures = journal.unread_failures;
        let text = render(&mut s, 100, 32);
        let last = text.lines().last().unwrap();
        assert!(last.contains(i18n::tr!(CheckingAltAlt)), "{text}");
        assert!(
            last.contains(&i18n::tf!(TaskUnreadFailuresCount, failures = 1)),
            "{text}"
        );
        assert!(last.contains(i18n::tr!(TasksOpenCommand)), "{text}");
        assert!(!last.contains("Alt+Enter"), "{text}");
        assert!(!text.contains("failure details"));
        let text = render(&mut s, 8, 4);
        assert!(
            text.lines()
                .last()
                .unwrap()
                .contains(i18n::tr!(CheckingAltAlt).chars().next().unwrap()),
            "{text}"
        );
        s.shutdown();
    }
}
