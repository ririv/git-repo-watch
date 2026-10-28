use super::*;
use engine::scheduler::timing::{Anchor, Basis, Plan, Source};

#[test]
fn dashboard_shows_the_basis_and_applied_next_time_even_when_the_draft_differs() {
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let (_home, mut s) = fixture(vec![]);
        let at = chrono::NaiveDate::from_ymd_opt(2030, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        let basis = Basis {
            source: Source::Explicit,
            at,
        };
        s.config.schedule_anchor = Anchor::Set(basis.clone());
        s.snapshot.schedule = Schedule::Anchored {
            plan: Plan::new(21600, basis).unwrap(),
        };
        command(&mut s, "schedule");
        for (width, height) in [(60, 18), (100, 28), (180, 36)] {
            let text = render(&mut s, width, height);
            assert!(
                text.contains(i18n::tr!(ScheduleAnchorLabel))
                    && text.contains(i18n::tr!(ScheduleNextLabel)),
                "{text}"
            );
            assert!(text.contains("2030-01-01 00:00:00"));
            assert!(text.contains(i18n::tr!(IntervalApplied)));
        }
        s.config.schedule_anchor = Anchor::Set(Basis {
            source: Source::Explicit,
            at: at + chrono::Duration::days(1),
        });
        let text = render(&mut s, 100, 28);
        assert!(text.contains("2030-01-02 00:00:00") && text.contains("2030-01-01 00:00:00"));
        assert!(text.contains(&i18n::tf!(IntervalPending, "6h")));
        s.snapshot.registration = Registration::NotLoaded;
        let text = render(&mut s, 100, 28);
        assert!(text.contains(i18n::tr!(ScheduleNextDisabled)));
        assert!(!text.contains("2030-01-01 00:00:00"));
    }
}

#[test]
fn anchor_editing_normalizes_the_draft_and_interval_changes_preserve_it() {
    let (_home, mut s) = fixture(vec![]);
    command(&mut s, "schedule");
    key(&mut s, KeyCode::Down);
    key(&mut s, KeyCode::Down);
    key(&mut s, KeyCode::Enter);
    assert_eq!(s.bar.text(), "/schedule anchor auto");
    key(&mut s, KeyCode::Esc);
    command(&mut s, "schedule anchor 00:00");
    assert!(matches!(s.mode, Mode::Confirm { .. }));
    key(&mut s, KeyCode::Esc);
    assert!(s.bar.text().starts_with("/schedule anchor 00:00"));
    key(&mut s, KeyCode::Enter);
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    let basis = s.service.store.config().unwrap().schedule_anchor;
    assert!(
        matches!(&basis, Anchor::Set(basis) if basis.source == Source::Explicit && basis.at.time() == chrono::NaiveTime::MIN)
    );
    command(&mut s, "schedule interval 1w");
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    let config = s.service.store.config().unwrap();
    assert_eq!(config.interval_seconds, 604800);
    assert_eq!(config.schedule_anchor, basis);
    assert!(s.service.store.report().unwrap().is_none());
}

#[test]
fn schedule_values_share_rows_with_aligned_action_buttons() {
    use crate::pointer::{Hit, Target};
    use ratatui::layout::Position;
    for language in [i18n::Language::Chinese, i18n::Language::English] {
        let _language = i18n::using(language);
        let (_home, mut s) = fixture(vec![]);
        command(&mut s, "schedule");
        for (width, height) in [(60, 18), (100, 28), (180, 36)] {
            let text = render(&mut s, width, height);
            let lines = text.lines().collect::<Vec<_>>();
            let mut columns = std::collections::BTreeSet::new();
            for (index, action) in Configuration::Schedule
                .actions(s.snapshot.registration)
                .iter()
                .enumerate()
            {
                let button = (0..height)
                    .flat_map(|y| (0..width).map(move |x| Position::new(x, y)))
                    .find(|point| {
                        s.pointer.hit(*point) == Some(Hit::Item(Target::ScheduleButton, index))
                    })
                    .unwrap();
                columns.insert(button.x);
                let label = match action {
                    Action::Interval => i18n::tr!(IntervalLabel),
                    Action::Anchor => i18n::tr!(ScheduleAnchorLabel),
                    Action::Workflow(_) => i18n::tr!(WorkflowScheduledActions),
                    Action::Apply => i18n::tr!(ApplicationLabel),
                    Action::EnableSchedule | Action::DisableSchedule => i18n::tr!(TaskStatusLabel),
                    _ => unreachable!(),
                };
                assert!(lines[button.y as usize].contains(label), "{text}");
            }
            assert_eq!(columns.len(), 1, "{text}");
            assert!(!text.contains(i18n::tr!(EditInterval)), "{text}");
        }
        s.setting_selected = Configuration::Schedule
            .actions(s.snapshot.registration)
            .len()
            - 1;
        render(&mut s, 28, 8);
        assert!(
            (0..8)
                .flat_map(|y| (0..28).map(move |x| Position::new(x, y)))
                .any(|point| s.pointer.hit(point)
                    == Some(Hit::Item(Target::ScheduleButton, s.setting_selected)))
        );
    }
}

#[test]
fn schedule_buttons_open_editors_with_one_click_and_keep_control_confirmation() {
    use crate::pointer::{Hit, Target};
    use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
    use ratatui::layout::Position;
    fn click(screen: &mut Screen, action: Action) {
        render(screen, 100, 28);
        let index = Configuration::Schedule
            .actions(screen.snapshot.registration)
            .iter()
            .position(|item| *item == action)
            .unwrap();
        let point = (0..28)
            .flat_map(|y| (0..100).map(move |x| Position::new(x, y)))
            .find(|point| {
                screen.pointer.hit(*point) == Some(Hit::Item(Target::ScheduleButton, index))
            })
            .unwrap();
        screen
            .mouse(
                MouseEvent {
                    kind: MouseEventKind::Down(MouseButton::Left),
                    column: point.x,
                    row: point.y,
                    modifiers: KeyModifiers::NONE,
                },
                &Cancel::default(),
            )
            .unwrap();
    }
    let (_home, mut s) = fixture(vec![]);
    command(&mut s, "schedule");
    click(&mut s, Action::Interval);
    assert_eq!(s.bar.text(), "/schedule interval 6h");
    key(&mut s, KeyCode::Esc);
    click(&mut s, Action::Anchor);
    assert_eq!(s.bar.text(), "/schedule anchor auto");
    key(&mut s, KeyCode::Esc);
    click(
        &mut s,
        Action::Workflow(engine::workflow::Trigger::Scheduled),
    );
    assert!(matches!(s.mode, Mode::Workflow(_)));
    key(&mut s, KeyCode::Esc);
    click(&mut s, Action::DisableSchedule);
    assert!(matches!(
        s.mode,
        Mode::Confirm {
            operation: Confirmation::Schedule(engine::scheduler::Control::Disable),
            ..
        }
    ));
    assert_eq!(s.snapshot.registration, Registration::Loaded);
    key(&mut s, KeyCode::Esc);
    click(&mut s, Action::Apply);
    assert!(matches!(
        s.mode,
        Mode::Confirm {
            operation: Confirmation::Schedule(engine::scheduler::Control::Apply),
            ..
        }
    ));
    key(&mut s, KeyCode::Esc);
    s.snapshot.registration = Registration::NotLoaded;
    click(&mut s, Action::EnableSchedule);
    assert!(matches!(
        s.mode,
        Mode::Confirm {
            operation: Confirmation::Schedule(engine::scheduler::Control::Enable),
            ..
        }
    ));
    key(&mut s, KeyCode::Esc);
    s.config.mouse.precise_click = false;
    click(&mut s, Action::Interval);
    assert!(s.bar.is_plain());
    assert!(matches!(s.mode, Mode::Browsing));
    assert!(s.service.store.report().unwrap().is_none());
}
