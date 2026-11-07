//! Explicit registration, discovery, and configuration mutations.
use super::*;
impl Service {
    pub fn add(&self, paths: &[PathBuf]) -> Result<usize> {
        let _lock = self.store.transaction(&Cancel::default())?;
        let mut config = self.store.config()?;
        let mut added = 0;
        for path in paths {
            let canonical = std::fs::canonicalize(path)?;
            local::open(&canonical)?;
            if !config.repositories.contains(&canonical) {
                config.repositories.push(canonical);
                added += 1
            }
        }
        self.store.save_config(&config)?;
        Ok(added)
    }
    pub fn configure(&self, operation: ConfigEdit) -> Result<()> {
        self.configure_with_cancel(operation, &Cancel::default())
    }
    pub fn configure_with_cancel(&self, operation: ConfigEdit, cancel: &Cancel) -> Result<()> {
        let _lock = self.store.transaction(cancel)?;
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
            ConfigEdit::Interval(value) => config.interval_seconds = value,
            ConfigEdit::Anchor(value) => config.schedule_anchor = value,
            ConfigEdit::Policy(value) => {
                use crate::workflow::Step;
                let mut steps = config.scheduled_actions.steps().to_vec();
                steps.retain(|step| *step != Step::Push);
                if value == PushPolicy::FastForward {
                    steps.insert(steps.len() - 1, Step::Push);
                }
                config.scheduled_actions = steps.try_into()?;
            }
            ConfigEdit::Concurrency(value) => config.max_concurrency = value,
            ConfigEdit::FetchOnStartup(value) => config.fetch_on_startup = value,
            ConfigEdit::ScheduledWorkflow(plan) => config.scheduled_actions = plan,
            ConfigEdit::FileIcons(value) => config.file_icons = value,
            ConfigEdit::TextWrap(value) => config.text_wrap = value,
            ConfigEdit::ChineseCommandTrigger(value) => config.chinese_command_trigger = value,
            ConfigEdit::Mouse(setting, value) => config.mouse.set(setting, value),
            ConfigEdit::Interaction(mode) => config.interaction_mode = mode,
            ConfigEdit::View(edit) => edit.apply(&mut config.view),
            ConfigEdit::Editor(editor) => config.editor = editor,
            ConfigEdit::AiEnabled(enabled) => config.ai.enabled = enabled,
            ConfigEdit::AiAgent(agent) => config.ai.agent = agent,
            ConfigEdit::AiModel { agent, model } => config.ai.program_mut_for(agent).model = model,
            ConfigEdit::AiPrompt(prompt) => config.ai.prompt = prompt,
        }
        self.store.save_config(&config)
    }
    pub fn scan(&self, request: &discovery::Request, cancel: &Cancel) -> Result<discovery::Report> {
        discovery::discover(&request.root, &request.excluded, cancel)
    }
}
