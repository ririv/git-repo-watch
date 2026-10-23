//! A persisted local-time basis defines every occurrence independently of execution.
use super::duration;
use anyhow::{Context, Result, ensure};
use chrono::{DateTime, Datelike, Duration, Local, NaiveDateTime, TimeZone, Timelike};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Anchor {
    Set(Basis),
    #[default]
    Unset,
}
impl Anchor {
    pub fn is_unset(&self) -> bool {
        matches!(self, Self::Unset)
    }
    pub fn basis(&self) -> Option<&Basis> {
        match self {
            Self::Set(basis) => Some(basis),
            Self::Unset => None,
        }
    }
    pub fn display(&self) -> String {
        match self {
            Self::Unset => i18n::tr!(ScheduleAnchorUnset).into(),
            Self::Set(basis) => {
                let at = basis.at.format("%Y-%m-%d %H:%M:%S");
                match basis.source {
                    Source::Automatic => format!("{at} ({})", i18n::tr!(ScheduleAnchorAutomatic)),
                    Source::Explicit => at.to_string(),
                }
            }
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    Automatic,
    Explicit,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Basis {
    pub source: Source,
    pub at: NaiveDateTime,
}
impl Basis {
    pub fn automatic(now: DateTime<Local>) -> Self {
        Self {
            source: Source::Automatic,
            at: now
                .naive_local()
                .with_nanosecond(0)
                .expect("zero nanoseconds"),
        }
    }
    pub fn validate(&self) -> Result<()> {
        ensure!(
            (1970..=9998).contains(&self.at.year()) && self.at.nanosecond() == 0,
            i18n::tr!(InvalidScheduleAnchor)
        );
        Ok(())
    }
}
pub fn parse_anchor(value: &str, today: chrono::NaiveDate) -> Result<Anchor> {
    let value = value.trim();
    if value == "auto" {
        return Ok(Anchor::Unset);
    }
    let at = [
        "%Y-%m-%d %H:%M:%S",
        "%Y-%m-%d %H:%M",
        "%Y-%m-%dT%H:%M:%S",
        "%Y-%m-%dT%H:%M",
    ]
    .iter()
    .find_map(|format| NaiveDateTime::parse_from_str(value, format).ok())
    .or_else(|| {
        ["%H:%M:%S", "%H:%M"]
            .iter()
            .find_map(|format| chrono::NaiveTime::parse_from_str(value, format).ok())
            .map(|at| today.and_time(at))
    })
    .context(i18n::tr!(InvalidScheduleAnchor))?;
    let basis = Basis {
        source: Source::Explicit,
        at,
    };
    basis.validate()?;
    // 不存在或重复的本地时间要求用户明确换一个时刻，不能猜测时区偏移。
    ensure!(
        Local.from_local_datetime(&at).single().is_some(),
        i18n::tr!(AmbiguousScheduleAnchor)
    );
    Ok(Anchor::Set(basis))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "RawPlan")]
pub struct Plan {
    interval_seconds: u64,
    basis: Basis,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawPlan {
    interval_seconds: u64,
    basis: Basis,
}
impl TryFrom<RawPlan> for Plan {
    type Error = anyhow::Error;
    fn try_from(raw: RawPlan) -> Result<Self> {
        Self::new(raw.interval_seconds, raw.basis)
    }
}
impl Plan {
    pub fn new(interval_seconds: u64, basis: Basis) -> Result<Self> {
        duration::validate(interval_seconds)?;
        basis.validate()?;
        Ok(Self {
            interval_seconds,
            basis,
        })
    }
    pub fn interval_seconds(&self) -> u64 {
        self.interval_seconds
    }
    pub fn basis(&self) -> &Basis {
        &self.basis
    }
    pub fn fingerprint(&self) -> Result<String> {
        Ok(super::digest(&serde_json::to_vec(self)?))
    }
    pub fn next_after<Tz: TimeZone>(&self, after: DateTime<Tz>) -> Result<DateTime<Tz>> {
        let elapsed =
            after.naive_local().and_utc().timestamp() - self.basis.at.and_utc().timestamp();
        let minimum = match self.basis.source {
            Source::Automatic => 1,
            Source::Explicit => 0,
        };
        let mut ordinal = (elapsed.div_euclid(self.interval_seconds as i64) + 1).max(minimum);
        loop {
            let seconds = ordinal
                .checked_mul(self.interval_seconds as i64)
                .context(i18n::tr!(InvalidScheduledInterval))?;
            let local = self
                .basis
                .at
                .checked_add_signed(Duration::seconds(seconds))
                .context(i18n::tr!(InvalidScheduledInterval))?;
            // 夏令时缺失的时刻跳过；重复的时刻只取第一次，避免同一个刻度执行两次。
            let candidate = after.timezone().from_local_datetime(&local).earliest();
            if let Some(candidate) = candidate
                && candidate > after
            {
                return Ok(candidate);
            }
            ordinal += 1;
        }
    }
    /// Native calendar events wake a short-lived dispatcher only in candidate minutes.
    pub fn occurrence_in_minute(&self, now: DateTime<Local>) -> Result<Option<DateTime<Local>>> {
        let start = now
            .with_second(0)
            .and_then(|t| t.with_nanosecond(0))
            .expect("minute boundary");
        let candidate = self.next_after(start - Duration::seconds(1))?;
        Ok((candidate < start + Duration::minutes(1)).then_some(candidate))
    }
}
