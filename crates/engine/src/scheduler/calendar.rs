//! Native wake-up calendars; due-time admission remains owned by the persisted plan.
use super::timing::Plan;
use chrono::{Datelike, Timelike};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Slot {
    pub weekday: Option<u32>,
    pub hour: Option<u32>,
    pub minute: Option<u32>,
}
pub fn slots(plan: &Plan) -> Vec<Slot> {
    let at = plan.basis().at;
    if plan.interval_seconds().is_multiple_of(604800) {
        return vec![Slot {
            weekday: Some(at.weekday().num_days_from_sunday()),
            hour: Some(at.hour()),
            minute: Some(at.minute()),
        }];
    }
    let mut a = plan.interval_seconds();
    let mut b = 86400;
    while b != 0 {
        (a, b) = (b, a % b);
    }
    if !a.is_multiple_of(60) || a == 60 {
        return vec![Slot {
            weekday: None,
            hour: None,
            minute: None,
        }];
    }
    let step = a / 60;
    if step <= 60 && 60u64.is_multiple_of(step) {
        return (u64::from(at.minute()) % step..60)
            .step_by(step as usize)
            .map(|minute| Slot {
                weekday: None,
                hour: None,
                minute: Some(minute as u32),
            })
            .collect();
    }
    let offset = (u64::from(at.hour()) * 60 + u64::from(at.minute())) % step;
    (offset..1440)
        .step_by(step as usize)
        .map(|minute| Slot {
            weekday: None,
            hour: Some((minute / 60) as u32),
            minute: Some((minute % 60) as u32),
        })
        .collect()
}
