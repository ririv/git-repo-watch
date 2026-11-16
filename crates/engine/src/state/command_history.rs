//! Bounded per-profile command history with atomic cross-process updates.
use super::{Lock, Store, atomic};
use anyhow::{Context, Result, ensure};
use std::{fs, io::ErrorKind};

const LIMIT: usize = 128;
const COMMAND_BYTES: usize = 16 * 1024;

impl Store {
    pub fn command_history(&self) -> Result<Vec<String>> {
        let path = self.state_dir().join("command-history.json");
        let data = match fs::read(&path) {
            Ok(data) => data,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => return Err(error).with_context(|| path.display().to_string()),
        };
        let entries: Vec<String> =
            serde_json::from_slice(&data).with_context(|| path.display().to_string())?;
        ensure!(entries.len() <= LIMIT, i18n::tr!(InvalidCommandHistory));
        for entry in &entries {
            validate(entry)?;
        }
        Ok(entries)
    }

    pub fn record_command(&self, command: &str) -> Result<()> {
        let command = command.trim();
        validate(command)?;
        self.prepare_state()?;
        let _lock = Lock::acquire(&self.state_dir().join("command-history.lock"))?;
        let mut entries = self.command_history()?;
        entries.retain(|entry| entry != command);
        entries.insert(0, command.to_owned());
        entries.truncate(LIMIT);
        atomic(
            &self.state_dir().join("command-history.json"),
            &serde_json::to_vec(&entries)?,
            0o600,
        )
    }
}

fn validate(command: &str) -> Result<()> {
    ensure!(
        command.starts_with('/')
            && command.len() > 1
            && command.len() <= COMMAND_BYTES
            && !command.contains('\0'),
        i18n::tr!(InvalidCommandHistory)
    );
    Ok(())
}
