//! Explicit registration, discovery, and configuration mutations.
use super::*;
impl Service {
    pub fn add(&self, paths: &[PathBuf]) -> Result<usize> {
        let _lock = self.store.lock()?;
        let mut config = self.store.config()?;
        let mut added = 0;
        for path in paths {
            let canonical = std::fs::canonicalize(path)?;
            local::open(&canonical)?;
            ensure!(
                !config
                    .excluded_roots
                    .iter()
                    .any(|x| canonical.starts_with(x)),
                i18n::tf!(RepositoryIsUnderAnExcludedDirectoryAlt, canonical.display())
            );
            if !config.repositories.contains(&canonical) {
                config.repositories.push(canonical);
                added += 1
            }
        }
        self.store.save_config(&config)?;
        Ok(added)
    }
    pub fn configure(&self, operation: ConfigEdit) -> Result<()> {
        let _lock = self.store.lock()?;
        let mut config = self.store.config()?;
        match operation {
            ConfigEdit::Remove(path) => {
                let path = state::canonical(&path)?;
                ensure!(
                    config.repositories.contains(&path),
                    i18n::tf!(NotInTheRepositoryList, path.display())
                );
                config.repositories.retain(|p| p != &path);
            }
            ConfigEdit::Exclude(path) => {
                let path = state::canonical(&path)?;
                config.repositories.retain(|p| !p.starts_with(&path));
                if !config.excluded_roots.contains(&path) {
                    config.excluded_roots.push(path)
                }
            }
            ConfigEdit::Unexclude(path) => {
                let path = state::canonical(&path)?;
                ensure!(
                    config.excluded_roots.contains(&path),
                    i18n::tr!(DirectoryIsNotInTheExclusionList)
                );
                config.excluded_roots.retain(|p| p != &path)
            }
            ConfigEdit::Interval(value) => config.interval_seconds = value,
            ConfigEdit::Policy(value) => config.push_policy = value,
            ConfigEdit::Concurrency(value) => config.max_concurrency = value,
            ConfigEdit::StartupCheck(value) => config.check_on_startup = value,
        }
        self.store.save_config(&config)
    }
    pub fn scan(&self, root: &Path, mode: ScanMode, cancel: &Cancel) -> Result<discovery::Report> {
        let _lock = self.store.lock()?;
        let mut config = self.store.config()?;
        let report = discovery::discover(root, &config.excluded_roots, cancel)?;
        state::write_json(&self.store.state_dir().join("import-report.json"), &report)?;
        ensure!(
            !report.repositories.is_empty(),
            i18n::tr!(NoGitRepositoriesFoundUnderThisDirectory)
        );
        if mode == ScanMode::Import {
            for record in &report.repositories {
                let path = record.path().to_owned();
                if !config.repositories.contains(&path) {
                    config.repositories.push(path)
                }
            }
            self.store.save_config(&config)?;
        }
        Ok(report)
    }
}
