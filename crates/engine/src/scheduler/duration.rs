//! Exact duration parsing and compact editable values.
use anyhow::{Result, ensure};

pub fn parse(value: &str) -> Result<u64> {
    let (number, multiplier) = [
        ("秒", 1u64),
        ("分钟", 60),
        ("分", 60),
        ("小时", 3600),
        ("天", 86400),
        ("周", 604800),
        ("s", 1),
        ("m", 60),
        ("h", 3600),
        ("d", 86400),
        ("w", 604800),
    ]
    .into_iter()
    .find_map(|(suffix, scale)| {
        value
            .trim()
            .strip_suffix(suffix)
            .map(|number| (number.trim(), scale))
    })
    .ok_or_else(|| anyhow::anyhow!(i18n::tr!(IntervalRequiresAUnitSMH)))?;
    let (whole, fraction) = number.split_once('.').unwrap_or((number, ""));
    ensure!(
        !whole.is_empty()
            && whole.bytes().all(|c| c.is_ascii_digit())
            && fraction.bytes().all(|c| c.is_ascii_digit())
            && fraction.len() <= 9,
        i18n::tr!(CheckIntervalMustBeAnIntegerOf)
    );
    let scale = 10u64.pow(fraction.len() as u32);
    let numerator = whole
        .parse::<u64>()?
        .checked_mul(scale)
        .and_then(|n| {
            n.checked_add(if fraction.is_empty() {
                0
            } else {
                fraction.parse().ok()?
            })
        })
        .and_then(|n| n.checked_mul(multiplier))
        .ok_or_else(|| anyhow::anyhow!(i18n::tr!(CheckIntervalMustBeAnIntegerOf)))?;
    ensure!(
        numerator.is_multiple_of(scale),
        i18n::tr!(CheckIntervalMustBeAnIntegerOf)
    );
    let seconds = numerator / scale;
    validate(seconds)?;
    Ok(seconds)
}

pub fn validate(seconds: u64) -> Result<()> {
    ensure!(
        (60..=i32::MAX as u64).contains(&seconds),
        i18n::tr!(CheckIntervalMustBeAnIntegerOf)
    );
    Ok(())
}

pub fn compact(seconds: u64) -> String {
    for (unit, scale) in [("w", 604800), ("d", 86400), ("h", 3600), ("m", 60)] {
        if seconds >= scale && seconds.is_multiple_of(scale) {
            return format!("{}{unit}", seconds / scale);
        }
    }
    format!("{seconds}s")
}
