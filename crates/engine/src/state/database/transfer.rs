//! Copy an explicitly selected legacy home without overwriting destination data.
use super::*;
use rusqlite::backup::{Backup, StepResult};
use serde::Serialize;
use std::{io::Write, path::PathBuf};

#[derive(Debug, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum Transfer {
    Configuration { config: PathBuf },
    Database { config: PathBuf, database: PathBuf },
}
impl Store {
    /// Convert source configuration and import its TOML with a consistent SQLite snapshot.
    pub fn migrate_from(&self, source: &Store, cancel: &Cancel) -> Result<Transfer> {
        cancel.check()?;
        self.prepare_state()?;
        source.prepare_state()?;
        ensure!(
            fs::canonicalize(self.state_dir())? != fs::canonicalize(source.state_dir())?,
            i18n::tr!(TransferSameSource)
        );
        let _target = self.lock()?;
        let _source = source.lock()?;
        let _batch = source.batch_lock()?;
        ensure!(
            !self.config_path().try_exists()?
                && !self.legacy_config_path().try_exists()?
                && !self.database_path().try_exists()?,
            i18n::tr!(TransferTargetExists)
        );
        // 占住全部源仓库并发槽位，正在执行的写操作会使导入立即报忙。
        let slots = source.state_dir().join("operation-slots");
        fs::create_dir_all(&slots)?;
        fs::set_permissions(&slots, fs::Permissions::from_mode(0o700))?;
        let _operations = (0..32)
            .map(|slot| Lock::acquire(&slots.join(format!("{slot}.lock"))))
            .collect::<Result<Vec<_>>>()?;
        source.migrate_configuration_locked(cancel)?;
        source.config()?;
        let configuration = fs::read(source.config_path())?;
        let database = Database::read(source)?;
        let directory = self.config_dir();
        fs::create_dir_all(&directory)?;
        if matches!(self, Store::Standard { .. }) {
            fs::set_permissions(&directory, fs::Permissions::from_mode(0o700))?;
        }
        let mut staged_config = tempfile::NamedTempFile::new_in(&directory)?;
        staged_config
            .as_file()
            .set_permissions(fs::Permissions::from_mode(0o600))?;
        staged_config.write_all(&configuration)?;
        staged_config.as_file().sync_all()?;
        let staged_database = database
            .as_ref()
            .map(|database| stage_database(self, database, cancel))
            .transpose()?;
        cancel.check()?;
        // 数据库先落盘，配置最后出现；失败时仅移除本次创建的目标，源 TOML 与数据库保留。
        if let Some(staged) = staged_database {
            staged.persist_noclobber(self.database_path())?;
        }
        if let Err(error) = staged_config.persist_noclobber(self.config_path()) {
            if database.is_some() {
                fs::remove_file(self.database_path())?;
            }
            return Err(error.into());
        }
        fs::File::open(directory)?.sync_all()?;
        fs::File::open(self.state_dir())?.sync_all()?;
        Ok(match database {
            Some(_) => Transfer::Database {
                config: self.config_path(),
                database: self.database_path(),
            },
            None => Transfer::Configuration {
                config: self.config_path(),
            },
        })
    }
}
fn stage_database(
    store: &Store,
    source: &Database,
    cancel: &Cancel,
) -> Result<tempfile::NamedTempFile> {
    let staged = tempfile::NamedTempFile::new_in(store.state_dir())?;
    staged
        .as_file()
        .set_permissions(fs::Permissions::from_mode(0o600))?;
    let mut destination = Connection::open(staged.path())?;
    {
        let backup = Backup::new(&source.connection, &mut destination)?;
        loop {
            cancel.check()?;
            match backup.step(128)? {
                StepResult::Done => break,
                StepResult::More => {}
                StepResult::Busy | StepResult::Locked => anyhow::bail!(crate::state::Busy),
                _ => anyhow::bail!(i18n::tr!(InvalidStateDatabase)),
            }
        }
    }
    let integrity: String =
        destination.query_row("PRAGMA integrity_check", [], |row| row.get(0))?;
    ensure!(integrity == "ok", i18n::tr!(InvalidStateDatabase));
    let foreign_keys: i64 =
        destination.query_row("SELECT count(*) FROM pragma_foreign_key_check", [], |row| {
            row.get(0)
        })?;
    ensure!(foreign_keys == 0, i18n::tr!(InvalidStateDatabase));
    // 关闭临时连接前明确合并日志，避免移动主文件时遗漏 WAL 中的页。
    destination.execute_batch("PRAGMA journal_mode=DELETE; PRAGMA journal_mode=WAL;")?;
    drop(destination);
    staged.as_file().sync_all()?;
    Ok(staged)
}
