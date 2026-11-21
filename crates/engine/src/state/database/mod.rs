//! Short SQLite transactions own persistent state; Git work stays outside this module.
mod reports;
mod schedule;
mod schema;
mod tasks;
mod transfer;
use super::{Lock, Store};
use crate::{cancel::Cancel, platform::files};
use anyhow::{Context, Result, ensure};
use rusqlite::{Connection, OpenFlags, Transaction, TransactionBehavior};
use std::{
    fs,
    path::Path,
    time::{Duration, Instant},
};
pub use transfer::Transfer;

pub(super) struct Database {
    connection: Connection,
}
impl Store {
    pub fn database_path(&self) -> std::path::PathBuf {
        self.state_dir().join("state.sqlite3")
    }
}

impl Database {
    pub(super) fn read(store: &Store) -> Result<Option<Self>> {
        Self::require_current_storage(store)?;
        if !store.database_path().try_exists()? {
            return Ok(None);
        }
        Self::checked(store).map(Some)
    }
    pub(super) fn write(store: &Store, cancel: &Cancel) -> Result<Self> {
        cancel.check()?;
        Self::require_current_storage(store)?;
        if !store.database_path().try_exists()? {
            store.prepare_state()?;
            let _migration = Lock::wait(&store.state_dir().join("migration.lock"), cancel)?;
            if !store.database_path().try_exists()? {
                Self::initialize(store, cancel)?;
            }
        }
        Self::checked(store)
    }
    fn connect(path: &Path) -> Result<Connection> {
        ensure!(
            fs::symlink_metadata(path)?.file_type().is_file(),
            i18n::tr!(InvalidStateDatabase)
        );
        files::private_path(path, 0o600)?;
        let connection = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .context(i18n::tr!(InvalidStateDatabase))?;
        connection.busy_timeout(Duration::from_millis(20))?;
        connection.execute_batch("PRAGMA foreign_keys=ON; PRAGMA synchronous=FULL;")?;
        Ok(connection)
    }
    fn checked(store: &Store) -> Result<Self> {
        let connection = Self::connect(&store.database_path())?;
        let id: i64 = connection.pragma_query_value(None, "application_id", |row| row.get(0))?;
        let version: i64 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
        ensure!(
            id == schema::APPLICATION_ID && version == schema::VERSION,
            i18n::tr!(UnsupportedStateDatabase)
        );
        let journal: String =
            connection.pragma_query_value(None, "journal_mode", |row| row.get(0))?;
        ensure!(journal == "wal", i18n::tr!(StateDatabaseRequiresWal));
        Ok(Self { connection })
    }
    fn transaction<T>(
        &mut self,
        cancel: &Cancel,
        write: impl FnOnce(&Transaction<'_>) -> Result<T>,
    ) -> Result<T> {
        let started = Instant::now();
        let transaction = loop {
            cancel.check()?;
            match self
                .connection
                .transaction_with_behavior(TransactionBehavior::Immediate)
            {
                Ok(transaction) => break transaction,
                Err(rusqlite::Error::SqliteFailure(error, _))
                    if matches!(
                        error.code,
                        rusqlite::ErrorCode::DatabaseBusy | rusqlite::ErrorCode::DatabaseLocked
                    ) && started.elapsed() < Duration::from_secs(5) => {}
                Err(error) => return Err(error.into()),
            }
        };
        let value = write(&transaction)?;
        cancel.check()?;
        transaction.commit()?;
        Ok(value)
    }
    fn require_current_storage(store: &Store) -> Result<()> {
        for name in [
            "latest.json",
            "repository-checks.json",
            "manual-commit.json",
            "manual-push.json",
            "import-report.json",
        ] {
            ensure!(
                !store.state_dir().join(name).try_exists()?,
                i18n::tr!(StorageMigrationRequired)
            );
        }
        Ok(())
    }
    fn initialize(store: &Store, cancel: &Cancel) -> Result<()> {
        let directory = store.state_dir();
        // 完成结构初始化并关闭 SQLite 连接后才发布数据库，避免并发打开半成品。
        let staged = tempfile::NamedTempFile::new_in(&directory)?;
        let mut database = Self {
            connection: Self::connect(staged.path())?,
        };
        database.transaction(cancel, |transaction| {
            transaction.execute_batch(schema::CREATE)?;
            transaction.pragma_update(None, "application_id", schema::APPLICATION_ID)?;
            transaction.pragma_update(None, "user_version", schema::VERSION)?;
            Ok(())
        })?;
        let mode: String =
            database
                .connection
                .pragma_update_and_check(None, "journal_mode", "WAL", |row| row.get(0))?;
        ensure!(mode == "wal", i18n::tr!(StateDatabaseRequiresWal));
        drop(database);
        staged.as_file().sync_all()?;
        cancel.check()?;
        files::publish(staged, &store.database_path(), false)?;
        files::sync_directory(&directory)?;
        Ok(())
    }
}

#[cfg(unix)]
fn path_key(path: &Path) -> &[u8] {
    path.as_os_str().as_encoded_bytes()
}
#[cfg(windows)]
fn path_key(path: &Path) -> Vec<u8> {
    use std::os::windows::ffi::OsStrExt;
    path.as_os_str()
        .encode_wide()
        .flat_map(u16::to_le_bytes)
        .collect()
}
