//! Monotonic stage measurements compose with task progress and existing task details.
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Stage {
    WaitingRepository,
    WaitingCapacity,
    InspectingRepository,
    FetchingRemote,
    ComparingBranches,
    CheckingWorktree,
    VerifyingRemote,
    PreparingPreview,
    RefreshingState,
    ReadingView,
}
impl Stage {
    pub fn label(self) -> &'static str {
        match self {
            Self::WaitingRepository => i18n::tr!(ProgressWaitingRepository),
            Self::WaitingCapacity => i18n::tr!(ProgressWaitingCapacity),
            Self::InspectingRepository => i18n::tr!(ProgressInspectingRepository),
            Self::FetchingRemote => i18n::tr!(ProgressFetchingRemote),
            Self::ComparingBranches => i18n::tr!(ProgressComparingBranches),
            Self::CheckingWorktree => i18n::tr!(ProgressCheckingWorktree),
            Self::VerifyingRemote => i18n::tr!(ProgressVerifyingRemote),
            Self::PreparingPreview => i18n::tr!(ProgressPreparingPreview),
            Self::RefreshingState => i18n::tr!(ProgressRefreshingState),
            Self::ReadingView => i18n::tr!(ProgressReadingView),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Measurement {
    pub stage: Stage,
    pub elapsed_ms: u128,
}
#[derive(Debug, Default)]
enum Active {
    #[default]
    Idle,
    Running {
        stage: Stage,
        started: Instant,
    },
}
#[derive(Debug, Default)]
pub struct Recorder {
    active: Active,
    measurements: Vec<Measurement>,
}
impl Recorder {
    pub fn enter(&mut self, stage: Stage) {
        self.enter_at(stage, Instant::now());
    }
    fn enter_at(&mut self, stage: Stage, now: Instant) {
        self.close(now);
        self.active = Active::Running {
            stage,
            started: now,
        };
    }
    fn close(&mut self, now: Instant) {
        if let Active::Running { stage, started } = std::mem::take(&mut self.active) {
            self.measurements.push(Measurement {
                stage,
                elapsed_ms: now.duration_since(started).as_millis(),
            });
        }
    }
    pub fn is_empty(&self) -> bool {
        matches!(self.active, Active::Idle) && self.measurements.is_empty()
    }
    pub fn finish(&mut self) -> Vec<Measurement> {
        self.close(Instant::now());
        std::mem::take(&mut self.measurements)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn finishing_preserves_monotonic_stage_durations_without_extra_stages() {
        let start = Instant::now();
        let mut recorder = Recorder::default();
        recorder.enter_at(Stage::WaitingRepository, start);
        recorder.enter_at(Stage::FetchingRemote, start + Duration::from_millis(250));
        recorder.close(start + Duration::from_millis(2750));
        assert_eq!(
            recorder.finish(),
            vec![
                Measurement {
                    stage: Stage::WaitingRepository,
                    elapsed_ms: 250
                },
                Measurement {
                    stage: Stage::FetchingRemote,
                    elapsed_ms: 2500
                },
            ]
        );
        assert!(recorder.is_empty());
    }
}
