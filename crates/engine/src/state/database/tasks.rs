//! Short task transactions preserve concurrent cancellation and acknowledgement.
use super::*;
use crate::tasks::{FailureRead, Journal, Outcome, Record, Resolution, State};
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
        self.recover_interrupted_tasks(&mut records)?;
        records.sort_by(|a, b| {
            a.priority()
                .cmp(&b.priority())
                .then_with(|| b.started_at.cmp(&a.started_at))
                .then_with(|| b.id.cmp(&a.id))
        });
        Ok(records)
    }

    pub fn task_journal(
        &self,
        history_limit: usize,
        selected_task: Option<&str>,
    ) -> Result<Journal> {
        ensure!(
            (1..=1000).contains(&history_limit),
            i18n::tr!(InvalidHistoryLimit)
        );
        let Some(db) = Database::read(self)? else {
            return Ok(Journal::default());
        };
        let mut journal = read_journal(&db, history_limit, selected_task)?;
        if self.recover_interrupted_tasks(&mut journal.records)? {
            journal = read_journal(&db, history_limit, selected_task)?;
        }
        Ok(journal)
    }

    fn recover_interrupted_tasks(&self, records: &mut [Record]) -> Result<bool> {
        let mut changed = false;
        // 先拿所有权锁再复读状态；不靠 PID 判断，避免进程号复用及完成时的竞态。
        for record in records {
            if !record.active() {
                continue;
            }
            match Lock::acquire(&self.task_lock_path(&record.id)) {
                Ok(_owner) => {
                    self.interrupt_task(&record.id)?;
                    *record = self.task(&record.id)?.context("missing task")?;
                    changed = true;
                }
                Err(error) if error.downcast_ref::<super::super::Busy>().is_some() => {}
                Err(error) => return Err(error),
            }
        }
        Ok(changed)
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
    pub fn read_task_failure(&self, id: &str) -> Result<()> {
        self.update_task(id, |record, _tx| {
            if let State::Finished {
                outcome: Outcome::Failed { read, .. },
                ..
            } = &mut record.state
                && *read == FailureRead::Unread
            {
                *read = FailureRead::Read {
                    at: crate::state::now(),
                };
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
                    outcome: Outcome::failure(i18n::tr!(TaskInterrupted)),
                };
            }
            Ok(())
        })
    }
}

fn read_journal(
    db: &Database,
    history_limit: usize,
    selected_task: Option<&str>,
) -> Result<Journal> {
    // 未读记录不受历史窗口限制。保留当前选中项，打开旧记录后详情仍可继续阅读。
    let mut query = db.connection.prepare(
        r#"
        WITH unread AS (
            SELECT id FROM tasks
            WHERE json_extract(record, '$.state.outcome.status') = 'failed'
                AND json_extract(record, '$.state.outcome.read.status') = 'unread'
        ),
        visible AS (
            SELECT id FROM tasks WHERE priority = 1 OR id = ?2
            UNION SELECT id FROM unread
            UNION SELECT id FROM (
                SELECT id FROM tasks WHERE priority <> 1 ORDER BY rowid DESC LIMIT ?1
            )
        )
        SELECT tasks.record, unread.id IS NOT NULL
        FROM visible JOIN tasks USING (id) LEFT JOIN unread USING (id)
        ORDER BY CASE WHEN tasks.priority = 1 THEN 0 WHEN unread.id IS NOT NULL THEN 1 ELSE 2 END,
            tasks.rowid DESC
        "#,
    )?;
    let mut journal = Journal::default();
    for row in query.query_map(params![history_limit as i64, selected_task], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, bool>(1)?))
    })? {
        let (json, unread_failure) = row?;
        journal.records.push(serde_json::from_str(&json)?);
        journal.unread_failures += usize::from(unread_failure);
    }
    Ok(journal)
}
