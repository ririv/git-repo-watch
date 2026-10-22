//! Legacy JSON is validated once, imported transactionally, then archived with digest checks.
use super::*;
use crate::{
    discovery,
    domain::{Report, ReportStatus, RepositoryResult},
    operations::commit::CommitResult,
    state::{Completion, Operation, OperationRecord},
};
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
const FILES: [&str; 5] = [
    "latest.json",
    "repository-checks.json",
    "manual-commit.json",
    "manual-push.json",
    "import-report.json",
];
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum Migration {
    Ready,
    Imported { files: usize },
}
enum ArchiveState {
    Pending,
    Archived,
}
impl rusqlite::types::FromSql for ArchiveState {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        match value.as_str()? {
            "pending" => Ok(Self::Pending),
            "archived" => Ok(Self::Archived),
            _ => Err(rusqlite::types::FromSqlError::InvalidType),
        }
    }
}
pub(super) fn has_legacy(store: &Store) -> Result<bool> {
    for name in FILES {
        if store.state_dir().join(name).try_exists()? {
            return Ok(true);
        }
    }
    Ok(false)
}
pub(super) struct File {
    name: &'static str,
    data: Vec<u8>,
    digest: String,
}
pub(super) struct Legacy {
    pub files: Vec<File>,
    batch: Option<Report>,
    repositories: BTreeMap<std::path::PathBuf, Report>,
    operations: Vec<OperationRecord>,
    scan: Option<discovery::Report>,
}
impl Legacy {
    pub fn read(store: &Store) -> Result<Self> {
        let mut files = Vec::new();
        for name in FILES {
            match fs::read(store.state_dir().join(name)) {
                Ok(data) => {
                    let digest = crate::scheduler::digest(&data);
                    files.push(File { name, data, digest });
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            }
        }
        let read = |name: &str| -> Result<Option<serde_json::Value>> {
            files
                .iter()
                .find(|file| file.name == name)
                .map(|file| {
                    serde_json::from_slice(&file.data)
                        .with_context(|| format!("{}: {name}", i18n::tr!(InvalidJson)))
                })
                .transpose()
        };
        let batch: Option<Report> = read("latest.json")?
            .map(serde_json::from_value)
            .transpose()?;
        if let Some(batch) = &batch {
            batch.validate()?;
            chrono::DateTime::parse_from_rfc3339(&batch.finished_at)?;
        }
        let mut repositories = BTreeMap::new();
        if let Some(batch) = &batch {
            for result in &batch.repositories {
                let path = result.path().to_owned();
                let mut single = batch.clone();
                single.repositories = vec![result.clone()];
                single.status = if matches!(result, RepositoryResult::Clean { .. }) {
                    ReportStatus::Clean
                } else {
                    ReportStatus::Attention
                };
                single.finished_at = batch
                    .repository_finished_at
                    .get(&path)
                    .unwrap_or(&batch.finished_at)
                    .clone();
                single.repository_finished_at.retain(|key, _| key == &path);
                single.repository_revisions.retain(|key, _| key == &path);
                if let Some(execution) = &mut single.execution {
                    execution.max_concurrency = 1;
                    execution.repositories.retain(|timing| timing.path == path);
                }
                repositories.insert(path, single);
            }
        }
        let independent: BTreeMap<std::path::PathBuf, Report> = read("repository-checks.json")?
            .map(serde_json::from_value)
            .transpose()?
            .unwrap_or_default();
        for (path, report) in independent {
            reports::validate_single(&report)?;
            ensure!(
                report.repositories[0].path() == path,
                i18n::tr!(InvalidRepositoryCheckReport)
            );
            if let Some(previous) = repositories.get(&path) {
                let order = |report: &Report| -> Result<_> {
                    Ok((
                        report.repository_revisions.get(&path).copied().unwrap_or(0),
                        chrono::DateTime::parse_from_rfc3339(&report.finished_at)?,
                    ))
                };
                let old = order(previous)?;
                let new = order(&report)?;
                if old.0 > 0 && old.0 == new.0 {
                    ensure!(
                        serde_json::to_value(&previous.repositories)?
                            == serde_json::to_value(&report.repositories)?,
                        i18n::tr!(LegacyStateConflict)
                    );
                }
                if new < old {
                    continue;
                }
            }
            repositories.insert(path, report);
        }
        for report in repositories.values() {
            reports::validate_single(report)?;
        }
        #[derive(Deserialize)]
        struct Commit {
            at: String,
            path: std::path::PathBuf,
            branch: String,
            result: CommitResult,
        }
        #[derive(Deserialize)]
        struct Push {
            at: String,
            path: std::path::PathBuf,
            push: crate::domain::Publication,
        }
        let mut operations = Vec::new();
        if let Some(value) = read("manual-commit.json")? {
            let old: Commit = serde_json::from_value(value)?;
            operations.push(OperationRecord {
                path: old.path,
                at: old.at,
                operation: Operation::Commit {
                    branch: old.branch,
                    completion: Completion::Succeeded { value: old.result },
                },
            });
        }
        if let Some(value) = read("manual-push.json")? {
            let old: Push = serde_json::from_value(value)?;
            operations.push(OperationRecord {
                path: old.path,
                at: old.at,
                operation: Operation::Push {
                    publication: old.push,
                },
            });
        }
        for record in &operations {
            ensure!(record.path.is_absolute(), i18n::tr!(InvalidStateDatabase));
            chrono::DateTime::parse_from_rfc3339(&record.at)?;
        }
        let scan = read("import-report.json")?
            .map(serde_json::from_value)
            .transpose()?;
        Ok(Self {
            files,
            batch,
            repositories,
            operations,
            scan,
        })
    }
    pub fn import(&self, transaction: &Transaction<'_>) -> Result<()> {
        if let Some(batch) = &self.batch {
            transaction.execute(
                "INSERT INTO batch_reports(finished_at,report) VALUES(?1,?2)",
                params![batch.finished_at, serde_json::to_string(batch)?],
            )?;
        }
        for (path, report) in &self.repositories {
            let mut report = report.clone();
            let revision = report.repository_revisions.get(path).copied().unwrap_or(1);
            ensure!(
                revision > 0 && revision <= i64::MAX as u64,
                i18n::tr!(InvalidRepositoryCheckReport)
            );
            report.repository_revisions.insert(path.clone(), revision);
            reports::insert_observation(transaction, &report, revision as i64)?;
        }
        for record in &self.operations {
            reports::insert_operation(transaction, record, None)?;
        }
        if let Some(scan) = &self.scan {
            transaction.execute(
                "INSERT INTO scan_reports(scanned_at,report) VALUES(?1,?2)",
                params![scan.scanned_at, serde_json::to_string(scan)?],
            )?;
        }
        for file in &self.files {
            transaction.execute(
                "INSERT INTO legacy_imports(name,digest,state) VALUES(?1,?2,'pending')",
                params![file.name, file.digest],
            )?;
        }
        Ok(())
    }
}
pub(super) fn archive(store: &Store, database: &mut Database, cancel: &Cancel) -> Result<usize> {
    let mut files = Vec::new();
    for name in FILES {
        let imported: Option<(String, ArchiveState)> = database
            .connection
            .query_row(
                "SELECT digest,state FROM legacy_imports WHERE name=?1",
                [name],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        let source = store.state_dir().join(name);
        match imported {
            None => ensure!(!source.try_exists()?, i18n::tr!(LegacyStateChanged)),
            Some((_, ArchiveState::Archived)) => {
                // 已完成归档后 SQLite 是唯一状态来源，备份可由用户删除；旧进程重建 JSON 仍须报错。
                ensure!(!source.try_exists()?, i18n::tr!(LegacyStateChanged));
            }
            Some((digest, ArchiveState::Pending)) => files.push((name, digest)),
        }
    }
    let directory = store.state_dir().join("legacy-json");
    if !files.is_empty() {
        fs::create_dir_all(&directory)?;
        fs::set_permissions(&directory, fs::Permissions::from_mode(0o700))?;
    }
    let mut archived = 0;
    for (name, digest) in files {
        cancel.check()?;
        let source = store.state_dir().join(name);
        let destination = directory.join(name);
        if source.try_exists()? {
            ensure!(
                crate::scheduler::digest(&fs::read(&source)?) == digest,
                i18n::tr!(LegacyStateChanged)
            );
            if destination.try_exists()? {
                ensure!(
                    crate::scheduler::digest(&fs::read(&destination)?) == digest,
                    i18n::tr!(LegacyStateConflict)
                );
                fs::remove_file(&source)?;
            } else {
                fs::rename(&source, &destination)?;
            }
            fs::File::open(&directory)?.sync_all()?;
            fs::File::open(store.state_dir())?.sync_all()?;
        }
        ensure!(
            destination.try_exists()?
                && crate::scheduler::digest(&fs::read(&destination)?) == digest,
            i18n::tr!(LegacyStateChanged)
        );
        fs::set_permissions(&destination, fs::Permissions::from_mode(0o600))?;
        database.transaction(cancel, |transaction| {
            transaction.execute(
                "UPDATE legacy_imports SET state='archived' WHERE name=?1",
                [name],
            )?;
            Ok(())
        })?;
        archived += 1;
    }
    Ok(archived)
}
