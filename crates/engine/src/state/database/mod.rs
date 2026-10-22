//! Short SQLite transactions own persistent state; Git work stays outside this module.
mod migration;
mod reports;
mod schema;
mod transfer;
use super::{Lock, Store};
use crate::cancel::Cancel;
use anyhow::{Context, Result, ensure};
pub use migration::Migration;
use rusqlite::{Connection, OpenFlags, Transaction, TransactionBehavior};
use std::{
    fs::{self, OpenOptions},
    os::unix::{
        ffi::OsStrExt,
        fs::{OpenOptionsExt, PermissionsExt},
    },
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
    /// Explicitly import legacy state and resume interrupted archival without discarding files.
    pub fn migrate(&self, cancel: &Cancel) -> Result<Migration> {
        let _configuration = self.transaction(cancel)?;
        self.prepare_state()?;
        let _migration = Lock::wait(&self.state_dir().join("migration.lock"), cancel)?;
        Database::initialize(self, cancel)
    }
}
impl Database {
    pub(super) fn read(store: &Store) -> Result<Option<Self>> {
        ensure!(
            !migration::has_legacy(store)?,
            i18n::tr!(StorageMigrationRequired)
        );
        if !store.database_path().try_exists()? {
            return Ok(None);
        }
        Self::wait_for_initialization(store, &Cancel::default())?;
        Self::checked(store).map(Some)
    }
    pub(super) fn write(store: &Store, cancel: &Cancel) -> Result<Self> {
        cancel.check()?;
        ensure!(
            !migration::has_legacy(store)?,
            i18n::tr!(StorageMigrationRequired)
        );
        if !store.database_path().try_exists()? {
            store.prepare_state()?;
            let _migration = Lock::wait(&store.state_dir().join("migration.lock"), cancel)?;
            Self::initialize(store, cancel)?;
        }
        Self::wait_for_initialization(store, cancel)?;
        Self::checked(store)
    }
    fn wait_for_initialization(store: &Store, cancel: &Cancel) -> Result<()> {
        let connection = Self::connect(store)?;
        let id: i64 = connection.pragma_query_value(None, "application_id", |row| row.get(0))?;
        let version: i64 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
        drop(connection);
        if version == 0 && id == 0 {
            let _migration = Lock::wait(&store.state_dir().join("migration.lock"), cancel)?;
            Self::initialize(store, cancel)?;
        }
        Ok(())
    }
    fn connect(store: &Store) -> Result<Connection> {
        let path = store.database_path();
        ensure!(
            fs::symlink_metadata(&path)?.file_type().is_file(),
            i18n::tr!(InvalidStateDatabase)
        );
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
        let connection = Connection::open_with_flags(
            &path,
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .context(i18n::tr!(InvalidStateDatabase))?;
        connection.busy_timeout(Duration::from_millis(20))?;
        connection.execute_batch("PRAGMA foreign_keys=ON; PRAGMA synchronous=FULL;")?;
        Ok(connection)
    }
    fn checked(store: &Store) -> Result<Self> {
        let connection = Self::connect(store)?;
        let id: i64 = connection.pragma_query_value(None, "application_id", |row| row.get(0))?;
        let version: i64 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
        ensure!(
            id == schema::APPLICATION_ID && version == schema::VERSION,
            i18n::tr!(UnsupportedStateDatabase)
        );
        let journal: String =
            connection.pragma_query_value(None, "journal_mode", |row| row.get(0))?;
        ensure!(journal == "wal", i18n::tr!(StateDatabaseRequiresWal));
        let pending: i64 = connection.query_row(
            "SELECT count(*) FROM legacy_imports WHERE state='pending'",
            [],
            |row| row.get(0),
        )?;
        ensure!(pending == 0, i18n::tr!(StorageMigrationRequired));
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
    fn initialize(store: &Store, cancel: &Cancel) -> Result<Migration> {
        let path = store.database_path();
        if !path.try_exists()? {
            OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(&path)?;
        }
        let connection = Self::connect(store)?;
        let version: i64 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
        let id: i64 = connection.pragma_query_value(None, "application_id", |row| row.get(0))?;
        let mut database = Self { connection };
        let migrated = if version == 0 && id == 0 {
            let tables: i64 = database.connection.query_row(
                "SELECT count(*) FROM sqlite_schema WHERE name NOT LIKE 'sqlite_%'",
                [],
                |row| row.get(0),
            )?;
            ensure!(tables == 0, i18n::tr!(InvalidStateDatabase));
            let legacy = migration::Legacy::read(store)?;
            let mode: String = database.connection.pragma_update_and_check(
                None,
                "journal_mode",
                "WAL",
                |row| row.get(0),
            )?;
            ensure!(mode == "wal", i18n::tr!(StateDatabaseRequiresWal));
            database.transaction(cancel, |transaction| {
                transaction.execute_batch(schema::CREATE)?;
                legacy.import(transaction)?;
                transaction.pragma_update(None, "application_id", schema::APPLICATION_ID)?;
                transaction.pragma_update(None, "user_version", schema::VERSION)?;
                Ok(())
            })?;
            legacy.files.len()
        } else {
            ensure!(
                version == schema::VERSION && id == schema::APPLICATION_ID,
                i18n::tr!(UnsupportedStateDatabase)
            );
            let journal: String =
                database
                    .connection
                    .pragma_query_value(None, "journal_mode", |row| row.get(0))?;
            ensure!(journal == "wal", i18n::tr!(StateDatabaseRequiresWal));
            0
        };
        let archived = migration::archive(store, &mut database, cancel)?;
        Ok(if migrated > 0 || archived > 0 {
            Migration::Imported {
                files: migrated.max(archived),
            }
        } else {
            Migration::Ready
        })
    }
}
fn path_key(path: &Path) -> &[u8] {
    path.as_os_str().as_bytes()
}
