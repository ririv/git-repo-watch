//! Agent names are parameter candidates supplied by the engine's supported-agent catalog.
use super::{argument_input, completion::Menu};
use crate::Editor;
use anyhow::{Result, ensure};
use engine::ai::Agent;

#[derive(Debug)]
pub(crate) struct Candidates {
    command: String,
    agents: Vec<Agent>,
    selected: usize,
    pub current: Agent,
}
impl Candidates {
    pub fn agents(&self) -> &[Agent] {
        &self.agents
    }
    pub fn selected(&self) -> usize {
        self.selected
    }
    pub fn advance(&mut self, backwards: bool) {
        self.selected =
            (self.selected + if backwards { self.agents.len() - 1 } else { 1 }) % self.agents.len();
    }
    pub fn choose(&self) -> Editor {
        Editor::new(format!(
            "{}{}",
            self.command,
            self.agents[self.selected].name()
        ))
    }
}
pub(super) fn complete(editor: &mut Editor, current: Agent) -> Result<Menu> {
    let (_, raw) = argument_input(editor.text.strip_prefix('/').expect("canonical command"))
        .expect("agent parameter");
    let start = editor.text.len() - raw.len();
    if editor.cursor < start {
        return Ok(Menu::Commands);
    }
    let prefix = raw[..editor.cursor - start].trim_start();
    let agents = Agent::ALL
        .iter()
        .copied()
        .filter(|agent| {
            agent
                .name()
                .get(..prefix.len())
                .is_some_and(|part| part.eq_ignore_ascii_case(prefix))
        })
        .collect::<Vec<_>>();
    ensure!(
        !agents.is_empty(),
        "{}: {prefix}; {}: {}",
        i18n::tr!(UnknownAgent),
        i18n::tr!(SupportedAgents),
        Agent::ALL
            .iter()
            .map(|agent| agent.name())
            .collect::<Vec<_>>()
            .join(", ")
    );
    let selected = agents
        .iter()
        .position(|agent| *agent == current)
        .unwrap_or(0);
    let candidates = Candidates {
        command: editor.text[..start].into(),
        agents,
        selected,
        current,
    };
    if candidates.agents.len() == 1 {
        *editor = candidates.choose();
        Ok(Menu::Commands)
    } else {
        Ok(Menu::Values(candidates))
    }
}
