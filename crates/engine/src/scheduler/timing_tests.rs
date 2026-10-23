use super::{calendar, duration, timing::*};
use chrono::{Duration, FixedOffset, TimeZone};

fn local(value: &str) -> chrono::NaiveDateTime {
    chrono::NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S").unwrap()
}
fn clock(value: &str) -> chrono::DateTime<FixedOffset> {
    FixedOffset::east_opt(8 * 3600)
        .unwrap()
        .from_local_datetime(&local(value))
        .unwrap()
}
fn plan(interval: &str, source: Source) -> Plan {
    Plan::new(
        duration::parse(interval).unwrap(),
        Basis {
            source,
            at: local("2026-10-23 00:00:00"),
        },
    )
    .unwrap()
}

#[test]
fn units_round_trip_exactly_and_do_not_round_partial_seconds() {
    for (input, seconds, compact) in [
        ("90s", 90, "90s"),
        ("30m", 1800, "30m"),
        ("6h", 21600, "6h"),
        ("1d", 86400, "1d"),
        ("2w", 1209600, "2w"),
        ("1.5h", 5400, "90m"),
        ("60 秒", 60, "1m"),
        ("30分钟", 1800, "30m"),
        ("6小时", 21600, "6h"),
        ("1天", 86400, "1d"),
        ("1周", 604800, "1w"),
    ] {
        assert_eq!(duration::parse(input).unwrap(), seconds);
        assert_eq!(duration::compact(seconds), compact);
        assert_eq!(duration::parse(compact).unwrap(), seconds);
    }
    for input in [
        "",
        "6",
        "abc",
        "6月",
        "59s",
        "NaNh",
        "0.00001d",
        "999999999999999999999w",
        "-1h",
        "1e2h",
    ] {
        assert!(duration::parse(input).is_err(), "{input}");
    }
}

#[test]
fn past_future_and_exact_boundaries_preserve_the_basis_instead_of_the_last_run() {
    let plan = plan("6h", Source::Explicit);
    for (now, next) in [
        ("2026-10-22 22:00:00", "2026-10-23 00:00:00"),
        ("2026-10-23 10:00:00", "2026-10-23 12:00:00"),
        ("2026-10-23 12:00:00", "2026-10-23 18:00:00"),
        ("2026-10-23 23:59:59", "2026-10-24 00:00:00"),
    ] {
        assert_eq!(plan.next_after(clock(now)).unwrap(), clock(next));
    }
    assert_eq!(
        plan.next_after(clock("2026-10-23 00:00:00") - Duration::nanoseconds(1))
            .unwrap(),
        clock("2026-10-23 00:00:00")
    );
    assert_eq!(
        serde_json::from_str::<Plan>(&serde_json::to_string(&plan).unwrap()).unwrap(),
        plan
    );
}

#[test]
fn automatic_basis_delays_the_first_run_and_long_intervals_keep_their_dates() {
    let automatic = plan("6h", Source::Automatic);
    assert_eq!(
        automatic.next_after(clock("2026-10-22 23:59:59")).unwrap(),
        clock("2026-10-23 06:00:00")
    );
    let two_days = plan("2d", Source::Explicit);
    assert_eq!(
        two_days.next_after(clock("2026-10-24 10:00:00")).unwrap(),
        clock("2026-10-25 00:00:00")
    );
    let weekly = plan("1w", Source::Explicit);
    assert_eq!(
        weekly.next_after(clock("2026-10-23 00:00:00")).unwrap(),
        clock("2026-10-30 00:00:00")
    );
    let changed = Plan::new(duration::parse("12h").unwrap(), automatic.basis().clone()).unwrap();
    assert_eq!(
        changed.next_after(clock("2026-10-24 07:00:00")).unwrap(),
        clock("2026-10-24 12:00:00")
    );
}

#[test]
fn native_candidates_align_to_the_basis_and_bound_the_number_of_wakeups() {
    let six = calendar::slots(&plan("6h", Source::Explicit));
    assert_eq!(
        six.iter()
            .map(|slot| (slot.hour, slot.minute))
            .collect::<Vec<_>>(),
        [
            (Some(0), Some(0)),
            (Some(6), Some(0)),
            (Some(12), Some(0)),
            (Some(18), Some(0))
        ]
    );
    let weekly = calendar::slots(&plan("1w", Source::Explicit));
    assert_eq!(weekly[0].weekday, Some(5));
    let seconds = calendar::slots(&plan("90s", Source::Explicit));
    assert_eq!(seconds.len(), 1);
    assert_eq!(seconds[0].minute, None);
    for interval in ["7m", "5h", "13h", "2d", "2w"] {
        assert!(calendar::slots(&plan(interval, Source::Explicit)).len() <= 24);
    }
}

#[test]
fn daylight_saving_transitions_skip_missing_and_repeated_local_occurrences() {
    const CHILD: &str = "APP_TIMING_DST_CHILD";
    if std::env::var_os(CHILD).is_none() {
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "scheduler::timing_tests::daylight_saving_transitions_skip_missing_and_repeated_local_occurrences", "--nocapture"])
            .env("TZ", "America/New_York").env(CHILD, "1").output().unwrap();
        assert!(
            output.status.success(),
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    }
    for (basis, now, expected) in [
        (
            "2026-03-08 00:30:00",
            "2026-03-08T01:30:00-05:00",
            "2026-03-08T03:30:00-04:00",
        ),
        (
            "2026-11-01 00:30:00",
            "2026-11-01T01:30:00-04:00",
            "2026-11-01T02:30:00-05:00",
        ),
    ] {
        let plan = Plan::new(
            3600,
            Basis {
                source: Source::Explicit,
                at: local(basis),
            },
        )
        .unwrap();
        let now = chrono::DateTime::parse_from_rfc3339(now)
            .unwrap()
            .with_timezone(&chrono::Local);
        assert_eq!(
            plan.next_after(now).unwrap(),
            chrono::DateTime::parse_from_rfc3339(expected).unwrap()
        );
    }
}
