//! Repository revisions, observations and operation records commit atomically.
use super::*;
use crate::{
    discovery,
    domain::Report,
    state::{OperationOrigin, OperationRecord, StoredOperation},
};
use rusqlite::{OptionalExtension, params};
use std::{collections::BTreeMap, num::NonZeroU64, os::unix::ffi::OsStringExt, path::PathBuf};

impl Store {
    pub fn report(&self) -> Result<Option<Report>> {
        let Some(database) = Database::read(self)? else {
            return Ok(None);
        };
        let json: Option<String> = database
            .connection
            .query_row(
                "SELECT report FROM batch_reports ORDER BY id DESC LIMIT 1",
                [],
                |row| row.get(0),
            )
            .optional()?;
        json.map(|json| decode_report(&json)).transpose()
    }
    pub fn save_report(&self, report: &Report) -> Result<()> {
        report.validate()?;
        let json = serde_json::to_string(report)?;
        Database::write(self, &Cancel::default())?.transaction(&Cancel::default(), |transaction| {
            transaction.execute(
                "INSERT INTO batch_reports(finished_at,report) VALUES(?1,?2)",
                params![report.finished_at, json],
            )?;
            Ok(())
        })
    }
    pub fn repository_report(&self, path: &Path) -> Result<Option<Report>> {
        let Some(database) = Database::read(self)? else {
            return Ok(None);
        };
        let json: Option<String> = database.connection.query_row("SELECT o.report FROM repository_heads h JOIN repository_observations o USING(path,revision) WHERE h.path=?1", [path_key(path)], |row| row.get(0)).optional()?;
        json.map(|json| decode_single(path, &json)).transpose()
    }
    pub fn repository_reports(&self) -> Result<BTreeMap<PathBuf, Report>> {
        let Some(database) = Database::read(self)? else {
            return Ok(BTreeMap::new());
        };
        let mut statement = database.connection.prepare("SELECT h.path,o.report FROM repository_heads h JOIN repository_observations o USING(path,revision)")?;
        let rows = statement.query_map([], |row| {
            Ok((row.get::<_, Vec<u8>>(0)?, row.get::<_, String>(1)?))
        })?;
        rows.map(|row| {
            let (path, json) = row?;
            let path = PathBuf::from(std::ffi::OsString::from_vec(path));
            let report = decode_single(&path, &json)?;
            Ok((path, report))
        })
        .collect()
    }
    pub fn save_repository_report(&self, report: &mut Report, cancel: &Cancel) -> Result<()> {
        self.save_observation(report, None, cancel)
    }
    pub fn save_operation_report(
        &self,
        report: &mut Report,
        operation: &OperationRecord,
        cancel: &Cancel,
    ) -> Result<()> {
        self.save_observation(report, Some(operation), cancel)
    }
    fn save_observation(
        &self,
        report: &mut Report,
        operation: Option<&OperationRecord>,
        cancel: &Cancel,
    ) -> Result<()> {
        validate_single(report)?;
        let path = report.repositories[0].path().to_owned();
        if let Some(operation) = operation {
            ensure!(
                operation.path == path,
                i18n::tr!(InvalidRepositoryCheckReport)
            );
        }
        let next = Database::write(self, cancel)?.transaction(cancel, |transaction| {
            let previous: i64 = transaction.query_row(
                "SELECT coalesce(max(revision),0) FROM repository_observations WHERE path=?1",
                [path_key(&path)],
                |row| row.get(0),
            )?;
            let revision = previous
                .checked_add(1)
                .context(i18n::tr!(InvalidRepositoryCheckReport))?;
            let mut next = report.clone();
            next.repository_revisions
                .insert(path.clone(), revision as u64);
            insert_observation(transaction, &next, revision)?;
            if let Some(operation) = operation {
                insert_operation(transaction, operation, Some(revision))?;
            }
            Ok(next)
        })?;
        *report = next;
        Ok(())
    }
    pub fn repository_history(&self, path: &Path, limit: usize) -> Result<Vec<Report>> {
        ensure!((1..=1000).contains(&limit), i18n::tr!(InvalidHistoryLimit));
        let Some(database) = Database::read(self)? else {
            return Ok(vec![]);
        };
        let mut statement=database.connection.prepare("SELECT report FROM repository_observations WHERE path=?1 ORDER BY revision DESC LIMIT ?2")?;
        let rows = statement.query_map(params![path_key(path), limit as i64], |row| {
            row.get::<_, String>(0)
        })?;
        rows.map(|row| decode_single(path, &row?)).collect()
    }
    pub fn operation_history(&self, path: &Path, limit: usize) -> Result<Vec<StoredOperation>> {
        ensure!((1..=1000).contains(&limit), i18n::tr!(InvalidHistoryLimit));
        let Some(database) = Database::read(self)? else {
            return Ok(vec![]);
        };
        let mut statement=database.connection.prepare("SELECT id,revision,record FROM operation_events WHERE path=?1 ORDER BY id DESC LIMIT ?2")?;
        let rows = statement.query_map(params![path_key(path), limit as i64], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, Option<i64>>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?;
        rows.map(|row| {
            let (id, revision, json) = row?;
            let origin = match revision {
                None => OperationOrigin::Imported,
                Some(value) => OperationOrigin::Observation {
                    revision: NonZeroU64::new(u64::try_from(value)?)
                        .context(i18n::tr!(InvalidStateDatabase))?,
                },
            };
            let record: OperationRecord = serde_json::from_str(&json)?;
            ensure!(record.path == path, i18n::tr!(InvalidStateDatabase));
            Ok(StoredOperation { id, origin, record })
        })
        .collect()
    }
    pub fn scan_report(&self) -> Result<Option<discovery::Report>> {
        let Some(database) = Database::read(self)? else {
            return Ok(None);
        };
        let json: Option<String> = database
            .connection
            .query_row(
                "SELECT report FROM scan_reports ORDER BY id DESC LIMIT 1",
                [],
                |row| row.get(0),
            )
            .optional()?;
        json.map(|value| serde_json::from_str(&value).map_err(Into::into))
            .transpose()
    }
    pub fn save_scan_report(&self, report: &discovery::Report, cancel: &Cancel) -> Result<()> {
        let json = serde_json::to_string(report)?;
        Database::write(self, cancel)?.transaction(cancel, |transaction| {
            transaction.execute(
                "INSERT INTO scan_reports(scanned_at,report) VALUES(?1,?2)",
                params![report.scanned_at, json],
            )?;
            Ok(())
        })
    }
}
pub(super) fn validate_single(report: &Report) -> Result<()> {
    report.validate()?;
    ensure!(
        report.repositories.len() == 1 && report.repositories[0].path().is_absolute(),
        i18n::tr!(InvalidRepositoryCheckReport)
    );
    chrono::DateTime::parse_from_rfc3339(&report.finished_at)
        .context(i18n::tr!(InvalidRepositoryCheckReport))?;
    Ok(())
}
fn decode_report(json: &str) -> Result<Report> {
    let report: Report = serde_json::from_str(json).context(i18n::tr!(InvalidReportFormat))?;
    report.validate()?;
    Ok(report)
}
fn decode_single(path: &Path, json: &str) -> Result<Report> {
    let report = decode_report(json)?;
    validate_single(&report)?;
    ensure!(
        report.repositories[0].path() == path,
        i18n::tr!(InvalidRepositoryCheckReport)
    );
    Ok(report)
}
pub(super) fn insert_observation(
    transaction: &Transaction<'_>,
    report: &Report,
    revision: i64,
) -> Result<()> {
    let path = report.repositories[0].path();
    transaction.execute(
        "INSERT INTO repository_observations(path,revision,finished_at,report) VALUES(?1,?2,?3,?4)",
        params![
            path_key(path),
            revision,
            report.finished_at,
            serde_json::to_string(report)?
        ],
    )?;
    transaction.execute("INSERT INTO repository_heads(path,revision) VALUES(?1,?2) ON CONFLICT(path) DO UPDATE SET revision=excluded.revision",params![path_key(path),revision])?;
    Ok(())
}
pub(super) fn insert_operation(
    transaction: &Transaction<'_>,
    record: &OperationRecord,
    revision: Option<i64>,
) -> Result<()> {
    chrono::DateTime::parse_from_rfc3339(&record.at)?;
    transaction.execute(
        "INSERT INTO operation_events(path,revision,at,kind,record) VALUES(?1,?2,?3,?4,?5)",
        params![
            path_key(&record.path),
            revision,
            record.at,
            record.operation.kind(),
            serde_json::to_string(record)?
        ],
    )?;
    Ok(())
}
