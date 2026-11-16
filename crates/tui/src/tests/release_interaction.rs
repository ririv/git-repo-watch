use super::*;
use engine::tasks::{Outcome as TaskOutcome, Resolution, Retry, Run};

fn buffer(s: &mut Screen, width: u16, height: u16) -> ratatui::buffer::Buffer {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal.draw(|frame| view::render(frame, s)).unwrap();
    terminal.backend().buffer().clone()
}

#[test]
fn current_feedback_stays_visible_with_historical_failures_at_every_width() {
    for language in [i18n::Language::Chinese, i18n::Language::English] {
        let _language = i18n::using(language);
        let (_home, mut s) = fixture(vec![]);
        let run = Run::begin(
            &s.service.store,
            crate::task_center::spec("older failure", None, Retry::Unavailable),
            &Cancel::default(),
        )
        .unwrap();
        run.finish(TaskOutcome::Failed {
            detail: "old error".into(),
            resolution: Resolution::Pending,
        })
        .unwrap();
        s.tasks.records = s.service.store.tasks(200).unwrap();
        s.message = "Saved settings".into();
        for width in [120, 80, 40, 16] {
            let text = render(&mut s, width, 24);
            let last = text.lines().last().unwrap();
            assert!(
                last.contains(if width >= 20 { "Saved settings" } else { "S" }),
                "{width}: {last}"
            );
            if width >= 80 {
                assert!(
                    last.contains(&i18n::tf!(TaskFailuresCount, pending = 1)),
                    "{last}"
                );
            }
        }
        render(&mut s, 4, 24);
        s.message.clear();
        let text = render(&mut s, 120, 24);
        assert!(
            text.lines()
                .last()
                .unwrap()
                .contains(i18n::tr!(TasksPending))
        );
        assert!(s.tasks.records[0].pending());
        s.shutdown();
    }
}

#[test]
fn enter_hints_follow_parameter_selection_navigation_and_execution_without_running_the_action() {
    for language in [i18n::Language::Chinese, i18n::Language::English] {
        let _language = i18n::using(language);
        let f = support::Fixture::create();
        let (_home, mut s) = fixture(vec![f.repo.clone()]);
        s.paste("//repo diff layout");
        assert!(
            s.command_enter_hint()
                .unwrap()
                .contains(i18n::tr!(CommandEnterParameters))
        );
        key(&mut s, KeyCode::Enter);
        let hint = s.command_enter_hint().unwrap();
        let choices = super::options::choices(&s);
        assert!(hint.contains(i18n::tr!(CommandEnterApply)));
        assert!(!hint.contains(choices.options[choices.selected].description.text()));
        key(&mut s, KeyCode::Down);
        assert_eq!(s.command_enter_hint().unwrap(), hint);
        assert!(render(&mut s, 120, 30).contains(&hint));
        assert_eq!(s.diff_preference, crate::diff::Preference::Auto);
        key(&mut s, KeyCode::Esc);
        key(&mut s, KeyCode::Esc);
        s.paste("//repo log");
        assert!(
            s.command_enter_hint()
                .unwrap()
                .contains(i18n::tr!(CommandEnterOpen))
        );
        key(&mut s, KeyCode::Esc);
        s.paste("//repo push");
        assert!(
            s.command_enter_hint()
                .unwrap()
                .contains(i18n::tr!(CommandEnterRun))
        );
        assert!(
            !s.command_enter_hint()
                .unwrap()
                .contains(i18n::CommandSummary::Push.text())
        );
        assert!(s.repository_tasks.is_empty());
        key(&mut s, KeyCode::Esc);
        s.paste("//repos add");
        key(&mut s, KeyCode::Enter);
        assert!(
            s.command_enter_hint()
                .unwrap()
                .contains(i18n::tr!(CommandEnterRequired))
        );
        let text = render(&mut s, 120, 30);
        assert!(text.contains(i18n::tr!(CommandEnterRequired)));
        s.shutdown();
    }
}

#[test]
fn recent_background_completion_stays_visible_beside_an_older_unresolved_failure() {
    let (_home, mut s) = fixture(vec![]);
    let failed = Run::begin(
        &s.service.store,
        crate::task_center::spec("older failure", None, Retry::Unavailable),
        &Cancel::default(),
    )
    .unwrap();
    failed
        .finish(TaskOutcome::Failed {
            detail: "old error".into(),
            resolution: Resolution::Pending,
        })
        .unwrap();
    let fresh = Run::begin(
        &s.service.store,
        crate::task_center::spec("fresh operation", None, Retry::Unavailable),
        &Cancel::default(),
    )
    .unwrap();
    s.tasks.records = s.service.store.tasks(200).unwrap();
    fresh
        .finish(TaskOutcome::Succeeded {
            detail: "done".into(),
        })
        .unwrap();
    let started = Instant::now();
    while !s
        .tasks
        .recent_notice()
        .is_some_and(|text| text.contains("fresh operation"))
    {
        assert!(started.elapsed() < Duration::from_secs(5));
        s.tasks_refresh_for_test();
        s.poll_tasks();
        thread::sleep(Duration::from_millis(5));
    }
    let text = render(&mut s, 120, 24);
    let last = text.lines().last().unwrap();
    assert!(last.contains("fresh operation"), "{last}");
    assert!(
        last.contains(&i18n::tf!(TaskFailuresCount, pending = 1)),
        "{last}"
    );
    assert!(s.tasks.records.iter().any(|record| record.pending()));
    s.shutdown();
}

#[test]
fn parameter_errors_are_red_in_both_languages_and_do_not_recolor_normal_notices() {
    use crate::appearance::Background;
    use ratatui::style::Color;
    for language in [i18n::Language::Chinese, i18n::Language::English] {
        let _language = i18n::using(language);
        let (_home, mut s) = fixture(vec![]);
        let config = std::fs::read(s.service.store.config_path()).unwrap();
        command(&mut s, "settings concurrency 0");
        assert!(s.message.is_error());
        for background in [Background::Light, Background::Dark, Background::Unreported] {
            s.background = background;
            for width in [100, 40, 16] {
                let output = buffer(&mut s, width, 24);
                assert_eq!(output[(2, 23)].fg, Color::LightRed);
                assert_eq!(
                    output[(2, 23)].symbol(),
                    s.message.chars().next().unwrap().to_string()
                );
            }
        }
        assert_eq!(
            std::fs::read(s.service.store.config_path()).unwrap(),
            config
        );
        command(&mut s, "repo diff layout unified");
        assert!(!s.message.is_error());
        let output = buffer(&mut s, 100, 24);
        assert_eq!(output[(2, 23)].fg, Color::Reset);
        s.shutdown();
    }
}

#[test]
fn fetching_a_repository_without_remotes_is_informational_not_an_error() {
    let f = support::Fixture::create();
    support::git(&f.repo, &["remote", "remove", "origin"]);
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo fetch");
    wait(&mut s);
    assert!(!s.message.is_error(), "{}", s.message);
    s.shutdown();
}

#[test]
fn one_current_error_wins_over_long_local_refresh_errors_and_failure_counts() {
    use unicode_width::UnicodeWidthStr;
    let path = PathBuf::from(format!(
        "/example/infinity/third-party/ghostty/vendor/{}/repo",
        "very-long-component/".repeat(12)
    ));
    let (_home, mut s) = fixture(vec![path.clone()]);
    s.live = crate::live::State::start();
    let crate::live::State::Active(controller) = &mut s.live else {
        panic!("local observer")
    };
    controller.errors.insert(
        path.clone(),
        "watch denied: complete background error".into(),
    );
    let failure = Run::begin(
        &s.service.store,
        crate::task_center::spec("older failure", None, Retry::Unavailable),
        &Cancel::default(),
    )
    .unwrap();
    failure
        .finish(TaskOutcome::Failed {
            detail: "old error".into(),
            resolution: Resolution::Pending,
        })
        .unwrap();
    let record = s.service.store.tasks(200).unwrap().remove(0);
    s.tasks.records = (0..84)
        .map(|index| {
            let mut value = record.clone();
            value.id = format!("older-{index}");
            value
        })
        .collect();
    command(&mut s, "settings concurrency 0");
    let error = s.message.to_string();
    for width in [160, 100, 60, 40, 20, 8] {
        let text = render(&mut s, width, 24);
        let last = text.lines().last().unwrap();
        assert!(
            last.contains(error.chars().next().unwrap()),
            "{width}: {last}"
        );
        assert!(!last.contains("ghostty"), "{last}");
        assert!(!last.contains("watch denied"), "{last}");
        assert!(!last.contains("! 84"), "{last}");
        assert!(last.width() <= usize::from(width), "{width}: {last}");
        if width >= 60 {
            assert!(last.contains(&error), "{last}");
        }
    }
    s.message.clear();
    let text = render(&mut s, 100, 24);
    let last = text.lines().last().unwrap();
    let prefix = i18n::tr!(LocalObservationFailed)
        .split(" · ")
        .next()
        .unwrap();
    assert!(last.contains(prefix), "{last}");
    assert!(!last.contains("! 84"), "{last}");
    s.snapshot.activity = engine::service::Activity::Running;
    let text = render(&mut s, 100, 24);
    let last = text.lines().last().unwrap();
    assert!(last.contains(i18n::tr!(CheckingAltAlt)), "{last}");
    assert!(!last.contains("ghostty"), "{last}");
    s.shutdown();
}

#[test]
fn task_summary_keeps_semantic_colors_when_standalone_or_appended_to_a_notice() {
    use crate::{appearance::Background, message::Message};
    use ratatui::style::Color;
    use unicode_width::UnicodeWidthStr;
    for language in [i18n::Language::Chinese, i18n::Language::English] {
        let _language = i18n::using(language);
        let (_home, mut s) = fixture(vec![]);
        let failure = Run::begin(
            &s.service.store,
            crate::task_center::spec("older failure", None, Retry::Unavailable),
            &Cancel::default(),
        )
        .unwrap();
        failure
            .finish(TaskOutcome::Failed {
                detail: "old error".into(),
                resolution: Resolution::Pending,
            })
            .unwrap();
        let record = s.service.store.tasks(200).unwrap().remove(0);
        s.tasks.records = (0..84)
            .map(|index| {
                let mut value = record.clone();
                value.id = format!("failed-{index}");
                value
            })
            .collect();
        let summary = format!(
            "{} · {}",
            i18n::tf!(TaskFailuresCount, pending = 84),
            i18n::tr!(TasksOpenCommand)
        );
        for background in [Background::Dark, Background::Light, Background::Unreported] {
            s.background = background;
            let colors = |screen: &mut Screen, summary: &str| {
                let rendered = render(screen, 200, 24);
                let text = rendered.lines().last().unwrap();
                let byte = text.find(summary).expect("complete task summary");
                let column = text[..byte].width();
                let output = buffer(screen, 200, 24);
                let row = &output.content[200 * 23..200 * 24];
                row[column..column + summary.width()]
                    .iter()
                    .filter(|cell| !cell.symbol().trim().is_empty())
                    .map(|cell| (cell.symbol().to_owned(), cell.fg))
                    .collect::<Vec<_>>()
            };
            s.message.clear();
            let standalone = colors(&mut s, &summary);
            assert_eq!(standalone[0].1, Color::Yellow);
            let text = render(&mut s, 200, 24);
            assert!(
                !text
                    .lines()
                    .last()
                    .unwrap()
                    .contains(&i18n::tf!(TaskRunningCount, running = 0)),
                "{text}"
            );
            assert_eq!(
                standalone
                    .iter()
                    .find(|(symbol, _)| symbol == "/")
                    .unwrap()
                    .1,
                Color::DarkGray
            );
            if language == i18n::Language::Chinese {
                assert_eq!(
                    standalone
                        .iter()
                        .find(|(symbol, _)| symbol == "任")
                        .unwrap()
                        .1,
                    Color::DarkGray
                );
            }
            for notice in [
                Message::error("Invalid parameter".into()),
                Message::information("Saved settings".into()),
            ] {
                s.message = notice;
                assert_eq!(colors(&mut s, &summary), standalone);
            }
            let active = Run::begin(
                &s.service.store,
                crate::task_center::spec("active task", None, Retry::Unavailable),
                &Cancel::default(),
            )
            .unwrap();
            s.tasks.records.push(active.record.clone());
            let active_summary = i18n::tf!(TasksSummary, pending = 84, running = 1);
            let active_colors = colors(&mut s, &active_summary);
            assert_eq!(
                active_colors
                    .iter()
                    .find(|(symbol, _)| symbol == "1")
                    .unwrap()
                    .1,
                Color::LightBlue
            );
            s.tasks.records.pop();
            active
                .finish(TaskOutcome::Cancelled {
                    detail: "test finished".into(),
                })
                .unwrap();
        }
        s.shutdown();
    }
}
