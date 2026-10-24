//! AI value labels and command annotations share the same saved configuration.
use super::*;

pub(super) fn model(config: &engine::ai::Config) -> String {
    if config.program().model.is_empty() {
        i18n::tr!(AgentDefault).into()
    } else {
        safe(&config.program().model)
    }
}

pub(super) fn annotation(action: Action, screen: &Screen) -> Option<String> {
    let value = match action {
        Action::AiModel => model(&screen.config.ai),
        Action::AiAgent => screen.config.ai.agent.label().into(),
        _ => return None,
    };
    Some(format!("{}: {value}", i18n::tr!(LayoutCurrent)))
}
