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
    key(&mut s, KeyCode::Enter);
    assert_eq!(s.bar.editor().text, "/schedule anchor auto");
    key(&mut s, KeyCode::Esc);
    command(&mut s, "schedule anchor 00:00");
    assert!(matches!(s.mode, Mode::Confirm { .. }));
    key(&mut s, KeyCode::Esc);
    assert!(s.bar.editor().text.starts_with("/schedule anchor 00:00"));
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
