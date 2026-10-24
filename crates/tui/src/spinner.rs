//! Clock-driven spinner animation with deterministic frame transitions.
use std::time::{Duration, Instant};
const FRAMES: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
#[derive(Debug)]
pub(super) struct Spinner {
    frame: usize,
    last_tick: Instant,
}
impl Default for Spinner {
    fn default() -> Self {
        Self::new(Instant::now())
    }
}
impl Spinner {
    pub(super) fn new(now: Instant) -> Self {
        Self {
            frame: 0,
            last_tick: now,
        }
    }
    pub(super) fn symbol(&self) -> &'static str {
        FRAMES[self.frame]
    }
    pub(super) fn tick(&mut self, now: Instant) -> bool {
        let steps = now.duration_since(self.last_tick).as_millis() / 100;
        if steps == 0 {
            return false;
        }
        self.frame = (self.frame + (steps % FRAMES.len() as u128) as usize) % FRAMES.len();
        self.last_tick += Duration::from_millis((steps * 100) as u64);
        true
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn animation_advances_on_ticks_and_can_complete_a_full_cycle_between_frames() {
        let start = Instant::now();
        let mut spinner = Spinner::new(start);
        assert!(!spinner.tick(start + Duration::from_millis(99)));
        assert_eq!(spinner.symbol(), "⠋");
        assert!(spinner.tick(start + Duration::from_millis(100)));
        assert_eq!(spinner.symbol(), "⠙");
        assert!(!spinner.tick(start + Duration::from_millis(199)));
        assert_eq!(spinner.symbol(), "⠙");
        assert!(spinner.tick(start + Duration::from_millis(1000)));
        assert_eq!(spinner.symbol(), "⠋");
        assert!(spinner.tick(start + Duration::from_millis(1100)));
        assert_eq!(spinner.symbol(), "⠙");
    }
}
