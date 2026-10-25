//! Delay loading feedback without delaying work or completed results.
use std::time::{Duration, Instant};

pub(crate) const FEEDBACK_DELAY: Duration = Duration::from_millis(150);

#[derive(Debug, Default)]
pub(crate) enum Feedback<T> {
    #[default]
    Idle,
    Quiet {
        since: Instant,
        previous: Option<T>,
    },
    Visible,
}

impl<T> Feedback<T> {
    pub fn begin(&mut self, previous: Option<T>, now: Instant) {
        let retained = match std::mem::take(self) {
            Self::Quiet { previous, .. } => previous,
            Self::Idle | Self::Visible => previous,
        };
        *self = Self::Quiet {
            since: now,
            previous: retained,
        };
    }
    pub fn quiet(&self) -> bool {
        matches!(self, Self::Quiet { .. })
    }
    pub fn previous(&self) -> Option<&T> {
        match self {
            Self::Quiet { previous, .. } => previous.as_ref(),
            Self::Idle | Self::Visible => None,
        }
    }
    pub fn previous_mut(&mut self) -> Option<&mut T> {
        match self {
            Self::Quiet { previous, .. } => previous.as_mut(),
            Self::Idle | Self::Visible => None,
        }
    }
    pub fn visible(&self) -> bool {
        matches!(self, Self::Visible)
    }
    pub fn tick(&mut self, now: Instant) -> bool {
        if matches!(self, Self::Quiet { since, .. } if now.duration_since(*since) >= FEEDBACK_DELAY)
        {
            *self = Self::Visible;
            return true;
        }
        false
    }
    pub fn finish(&mut self) -> bool {
        let changed = !matches!(self, Self::Idle);
        *self = Self::Idle;
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn slow_reads_show_feedback_at_the_deadline_and_initial_reads_stay_quiet_before_it() {
        let start = Instant::now();
        let mut feedback = Feedback::<String>::default();
        feedback.begin(None, start);
        assert!(!feedback.visible());
        assert!(!feedback.tick(start + FEEDBACK_DELAY - Duration::from_millis(1)));
        assert!(feedback.tick(start + FEEDBACK_DELAY));
        assert!(feedback.visible());
        assert!(!feedback.tick(start + FEEDBACK_DELAY + Duration::from_secs(1)));
        assert!(feedback.finish());
        assert!(!feedback.finish());
    }

    #[test]
    fn completed_reads_release_the_previous_view_immediately_without_a_minimum_wait() {
        let start = Instant::now();
        let previous = Arc::new("previous");
        let weak = Arc::downgrade(&previous);
        let mut feedback = Feedback::default();
        feedback.begin(Some(previous), start);
        assert!(feedback.previous().is_some());
        assert!(!feedback.tick(start + Duration::from_millis(10)));
        feedback.finish();
        assert!(weak.upgrade().is_none());
        assert!(!feedback.visible());
    }

    #[test]
    fn rapid_reselection_retains_one_complete_view_and_resets_the_deadline() {
        let start = Instant::now();
        let previous = Arc::new("original");
        let weak = Arc::downgrade(&previous);
        let mut feedback = Feedback::default();
        feedback.begin(Some(previous), start);
        for step in 1..10 {
            let intermediate = Arc::new("intermediate");
            let intermediate_weak = Arc::downgrade(&intermediate);
            feedback.begin(Some(intermediate), start + Duration::from_millis(step * 10));
            assert!(intermediate_weak.upgrade().is_none());
            assert_eq!(feedback.previous().unwrap().as_ref(), &"original");
        }
        assert!(!feedback.tick(start + FEEDBACK_DELAY));
        assert!(feedback.tick(start + Duration::from_millis(90) + FEEDBACK_DELAY));
        assert!(weak.upgrade().is_none());
    }
}
