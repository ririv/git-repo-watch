//! Monotonic per-repository deadlines, independent of terminal and worker lifetimes.
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

#[derive(Default)]
pub(super) enum Clock {
    #[default]
    Disabled,
    Enabled {
        interval: Duration,
        repositories: BTreeMap<PathBuf, Deadline>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn deadlines_follow_completion_and_interval_changes_without_catchup() {
        let at = Instant::now();
        let a = PathBuf::from("/a");
        let b = PathBuf::from("/b");
        let mut clock = Clock::default();
        clock.configure(true, 180, &[a.clone(), b.clone()]);
        assert_eq!(clock.due(at), [a.clone(), b.clone()]);
        clock.started(&a);
        clock.started(&b);
        assert!(clock.due(at + Duration::from_secs(600)).is_empty());
        clock.finished(&a, at);
        clock.finished(&b, at + Duration::from_secs(40));
        assert!(clock.due(at + Duration::from_secs(179)).is_empty());
        assert_eq!(clock.due(at + Duration::from_secs(180)), [a.clone()]);
        clock.started(&a);
        clock.finished(&a, at + Duration::from_secs(1000));
        assert_eq!(clock.due(at + Duration::from_secs(1000)), [b.clone()]);
        clock.configure(true, 60, &[a.clone()]);
        assert!(clock.due(at + Duration::from_secs(1059)).is_empty());
        assert_eq!(clock.due(at + Duration::from_secs(1060)), [a]);
    }
    #[test]
    fn disabling_and_reenabling_start_a_new_schedule_without_old_repositories() {
        let now = Instant::now();
        let mut clock = Clock::default();
        let paths = vec![PathBuf::from("/a")];
        clock.configure(false, 180, &paths);
        assert!(clock.due(now).is_empty());
        clock.configure(true, 180, &paths);
        clock.started(&paths[0]);
        clock.configure(false, 180, &paths);
        assert!(clock.due(now).is_empty());
        clock.configure(true, 180, &[PathBuf::from("/b")]);
        assert_eq!(clock.due(now), [PathBuf::from("/b")]);
    }
}
pub(super) enum Deadline {
    Due,
    Fetching,
    Waiting(Instant),
}
impl Clock {
    pub fn configure(&mut self, enabled: bool, seconds: u64, paths: &[PathBuf]) {
        if !enabled {
            *self = Self::Disabled;
            return;
        }
        if matches!(self, Self::Disabled) {
            *self = Self::Enabled {
                interval: Duration::from_secs(seconds),
                repositories: BTreeMap::new(),
            };
        }
        let Self::Enabled {
            interval,
            repositories,
        } = self
        else {
            unreachable!()
        };
        *interval = Duration::from_secs(seconds);
        let active = paths.iter().collect::<BTreeSet<_>>();
        repositories.retain(|path, _| active.contains(path));
        for path in paths {
            repositories.entry(path.clone()).or_insert(Deadline::Due);
        }
    }
    pub fn started(&mut self, path: &Path) {
        if let Self::Enabled { repositories, .. } = self
            && let Some(deadline) = repositories.get_mut(path)
        {
            *deadline = Deadline::Fetching;
        }
    }
    pub fn finished(&mut self, path: &Path, at: Instant) {
        if let Self::Enabled { repositories, .. } = self
            && let Some(deadline) = repositories.get_mut(path)
        {
            *deadline = Deadline::Waiting(at);
        }
    }
    pub fn due(&self, now: Instant) -> Vec<PathBuf> {
        match self {
            Self::Disabled => vec![],
            Self::Enabled {
                interval,
                repositories,
            } => repositories
                .iter()
                .filter_map(|(path, state)| {
                    let ready = match state {
                        Deadline::Due => true,
                        Deadline::Fetching => false,
                        Deadline::Waiting(at) => now.saturating_duration_since(*at) >= *interval,
                    };
                    ready.then(|| path.clone())
                })
                .collect(),
        }
    }
}
