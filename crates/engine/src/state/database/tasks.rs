//! Short task transactions preserve concurrent cancellation and acknowledgement.
use super::*;
use crate::tasks::{Outcome, Record, Resolution, State};
use rusqlite::{OptionalExtension, params};
impl Store {
    pub(crate) fn task_lock_path(&self, id: &str) -> std::path::PathBuf {
        use sha2::{Digest, Sha256};
        self.state_dir()
            .join(format!("task-{:x}.lock", Sha256::digest(id)))
    }
    pub(crate) fn insert_task(&self, record: &Record) -> Result<()> {
        Database::write(self, &Cancel::default())?.transaction(&Cancel::default(), |tx| {
            tx.execute(
                "INSERT INTO tasks(id,priority,started_at,record) VALUES(?1,?2,?3,?4)",
                params![
                    record.id,
                    record.priority(),
                    record.started_at,
                    serde_json::to_string(record)?
                ],
            )?;
            Ok(())
        })
    }
    pub fn task(&self, id: &str) -> Result<Option<Record>> {
        let Some(db) = Database::read(self)? else {
            return Ok(None);
        };
        let json: Option<String> = db
            .connection
            .query_row("SELECT record FROM tasks WHERE id=?1", [id], |row| {
                row.get(0)
            })
            .optional()?;
        json.map(|value| serde_json::from_str(&value).map_err(Into::into))
            .transpose()
    }
    pub fn tasks(&self, history_limit: usize) -> Result<Vec<Record>> {
        ensure!(
            (1..=1000).contains(&history_limit),
            i18n::tr!(InvalidHistoryLimit)
        );
        let Some(db) = Database::read(self)? else {
            return Ok(vec![]);
        };
        let mut query = db.connection.prepare("SELECT record FROM tasks WHERE priority<2 UNION ALL SELECT record FROM (SELECT record FROM tasks WHERE priority=2 ORDER BY started_at DESC,id DESC LIMIT ?1)")?;
        let mut records = query
            .query_map([history_limit as i64], |row| row.get::<_, String>(0))?
            .map(|json| Ok(serde_json::from_str::<Record>(&json?)?))
            .collect::<Result<Vec<_>>>()?;
        // 先拿所有权锁再复读状态；不靠 PID 判断，避免进程号复用及完成时的竞态。
        for record in &mut records {
            if !record.active() {
                continue;
            }
            match Lock::acquire(&self.task_lock_path(&record.id)) {
                Ok(_owner) => {
                    self.interrupt_task(&record.id)?;
                    *record = self.task(&record.id)?.context("missing task")?;
                }
                Err(error) if error.downcast_ref::<super::super::Busy>().is_some() => {}
                Err(error) => return Err(error),
            }
        }
        records.sort_by(|a, b| {
            a.priority()
                .cmp(&b.priority())
                .then_with(|| b.started_at.cmp(&a.started_at))
                .then_with(|| b.id.cmp(&a.id))
        });
        Ok(records)
    }
    fn update_task(
        &self,
        id: &str,
        change: impl FnOnce(&mut Record, &Transaction<'_>) -> Result<()>,
    ) -> Result<()> {
        Database::write(self, &Cancel::default())?.transaction(&Cancel::default(), |tx| {
            let json: String =
                tx.query_row("SELECT record FROM tasks WHERE id=?1", [id], |row| {
                    row.get(0)
                })?;
            let mut record: Record = serde_json::from_str(&json)?;
            change(&mut record, tx)?;
            tx.execute(
                "UPDATE tasks SET priority=?2,record=?3 WHERE id=?1",
                params![id, record.priority(), serde_json::to_string(&record)?],
            )?;
            Ok(())
        })
    }
    pub(crate) fn task_progress(&self, id: &str, progress: &str) -> Result<()> {
        self.update_task(id, |record, _tx| {
            match &mut record.state {
                State::Running { progress: value } | State::Cancelling { progress: value } => {
                    *value = progress.into()
                }
                State::Finished { .. } => {}
            }
            Ok(())
        })
    }
    pub fn cancel_task(&self, id: &str) -> Result<()> {
        self.update_task(id, |record, _tx| {
            match &record.state {
                State::Running { progress } => {
                    record.state = State::Cancelling {
                        progress: progress.clone(),
                    }
                }
                State::Cancelling { .. } => {}
                State::Finished { .. } => anyhow::bail!("task has already finished"),
            }
            Ok(())
        })
    }
    pub fn resolve_task(&self, id: &str) -> Result<()> {
        self.update_task(id, |record, _tx| {
            match &mut record.state {
                State::Finished {
                    outcome: Outcome::Failed { resolution, .. },
                    ..
                } => {
                    *resolution = Resolution::Handled {
                        at: crate::state::now(),
                    }
                }
                _ => anyhow::bail!("only failed tasks can be marked handled"),
            }
            Ok(())
        })
    }
    pub(crate) fn finish_task(
        &self,
        id: &str,
        outcome: Outcome,
        timings: Vec<crate::tasks::progress::Measurement>,
    ) -> Result<()> {
        self.update_task(id, |record, tx| {
            ensure!(record.active(), "task has already finished");
            if matches!(outcome, Outcome::Succeeded { .. })
                && let Some(parent) = &record.spec.retry_of
            {
                let json: String =
                    tx.query_row("SELECT record FROM tasks WHERE id=?1", [parent], |row| {
                        row.get(0)
                    })?;
                let mut original: Record = serde_json::from_str(&json)?;
                if let State::Finished {
                    outcome: Outcome::Failed { resolution, .. },
                    ..
                } = &mut original.state
                {
                    *resolution = Resolution::Handled {
                        at: crate::state::now(),
                    };
                    tx.execute(
                        "UPDATE tasks SET priority=?2,record=?3 WHERE id=?1",
                        params![
                            parent,
                            original.priority(),
                            serde_json::to_string(&original)?
                        ],
                    )?;
                }
            }
            record.state = State::Finished {
                at: crate::state::now(),
                outcome,
            };
            record.stage_timings = timings;
            Ok(())
        })
    }
    fn interrupt_task(&self, id: &str) -> Result<()> {
        self.update_task(id, |record, _tx| {
            if record.active() {
                record.state = State::Finished {
                    at: crate::state::now(),
                    outcome: Outcome::Failed {
                        detail: i18n::tr!(TaskInterrupted).into(),
                        resolution: Resolution::Pending,
                    },
                };
            }
            Ok(())
        })
    }
}
/// Import previous operation history without inferring whether a failure was handled.
pub(super) fn import_operations(tx: &Transaction<'_>) -> Result<()> {
    use crate::state::{Completion, Operation, OperationRecord};
    use crate::tasks::{Retry, Scope, Source, Spec};
    fn completion<T>(value: &Completion<T>, detail: String) -> Outcome {
        match value {
            Completion::Succeeded { .. } => Outcome::Succeeded { detail },
            Completion::Failed { error } => Outcome::Failed {
                detail: error.clone(),
                resolution: Resolution::Pending,
            },
            Completion::Cancelled { error } => Outcome::Cancelled {
                detail: error.clone(),
            },
        }
    }
    let mut query = tx.prepare("SELECT id,record FROM operation_events ORDER BY id")?;
    let rows = query.query_map([], |row| {
        Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
    })?;
    for row in rows {
        let (id, json) = row?;
        let operation: OperationRecord = serde_json::from_str(&json)?;
        let detail = serde_json::to_string_pretty(&operation.operation)?;
        let outcome = match &operation.operation {
            Operation::Stage { completion: value }
            | Operation::Unstage { completion: value }
            | Operation::PullFetch { completion: value } => completion(value, detail),
            Operation::Commit {
                completion: value, ..
            } => completion(value, detail),
            Operation::Workspace {
                completion: value, ..
            } => completion(value, detail),
            Operation::Push {
                publication: crate::domain::Publication::Pushed { .. },
            } => Outcome::Succeeded { detail },
            Operation::Push {
                publication: crate::domain::Publication::Failed { error, .. },
            } => Outcome::Failed {
                detail: error.clone(),
                resolution: Resolution::Pending,
            },
        };
        let record = Record {
            id: format!("operation-{id}"),
            stage_timings: vec![],
            spec: Spec {
                title: operation.operation.kind().into(),
                scope: Scope::Repository {
                    path: operation.path,
                },
                source: Source::Imported,
                retry: Retry::Unavailable,
                retry_of: None,
                operation: None,
            },
            started_at: operation.at.clone(),
            state: State::Finished {
                at: operation.at,
                outcome,
            },
        };
        tx.execute(
            "INSERT INTO tasks(id,priority,started_at,record) VALUES(?1,?2,?3,?4)",
            params![
                record.id,
                record.priority(),
                record.started_at,
                serde_json::to_string(&record)?
            ],
        )?;
    }
    Ok(())
}
