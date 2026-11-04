//! Settings-page entries can open command editors or update preferences directly.
use crate::{Action, Configuration, Effect, Outcome, Screen, command};
use anyhow::Result;
use engine::{cancel::Cancel, scheduler::Registration, service::ConfigEdit};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Item {
    Command(Action),
    ChineseCommandTrigger,
    FileIcons,
    Interaction,
    AiEnabled,
    Mouse(engine::state::MouseSetting),
}
impl Item {
    pub fn context(self) -> command::Context {
        match self {
            Self::Command(
                Action::StartupCheck | Action::Workflow(engine::workflow::Trigger::Startup),
            ) => command::Context::Startup,
            Self::AiEnabled
            | Self::Command(Action::AiAgent | Action::AiModel | Action::AiPrompt) => {
                command::Context::Ai
            }
            _ => command::Context::Settings,
        }
    }
    pub fn description(self) -> &'static str {
        match self {
            Self::Command(action) => command::description(action),
            Self::Interaction => i18n::tr!(InteractionSetting),
            Self::AiEnabled => i18n::tr!(AiEnabled),
            Self::FileIcons => i18n::tr!(FileIcons),
            Self::ChineseCommandTrigger => i18n::tr!(ChineseCommandTrigger),
            Self::Mouse(setting) => match setting {
                engine::state::MouseSetting::PreciseClick => i18n::tr!(MousePreciseClick),
                engine::state::MouseSetting::Scroll => i18n::tr!(MouseScroll),
                engine::state::MouseSetting::SelectPane => i18n::tr!(MouseSelectPane),
            },
        }
    }
}
impl Configuration {
    pub(crate) fn items(self, registration: Registration, ai: &engine::ai::Config) -> Vec<Item> {
        let mut items = self
            .actions(registration)
            .iter()
            .copied()
            .map(Item::Command)
            .collect::<Vec<_>>();
        if self == Self::General {
            items.push(Item::FileIcons);
            items.push(Item::Command(Action::EditorSetting));
        }
        if self == Self::General && i18n::language() == i18n::Language::Chinese {
            items.push(Item::ChineseCommandTrigger);
        }
        if self == Self::General {
            items.push(Item::Interaction);
            use engine::state::MouseSetting::*;
            items.extend([PreciseClick, Scroll, SelectPane].map(Item::Mouse));
            items.push(Item::AiEnabled);
            if ai.enabled {
                items.extend(
                    [Action::AiAgent, Action::AiModel, Action::AiPrompt].map(Item::Command),
                );
            }
        }
        items
    }
}
impl Screen {
    pub(crate) fn activate_setting(&mut self, item: Item, root: &Cancel) -> Result<bool> {
        match item {
            Item::Command(action) => self.activate(
                action,
                self.selected_path(),
                crate::clipboard::Selection::None,
                root,
            ),
            Item::FileIcons => {
                let enabled = !self.config.file_icons;
                self.start(
                    item.description(),
                    Effect::Configuration,
                    move |service, cancel, _| {
                        service.configure_with_cancel(ConfigEdit::FileIcons(enabled), &cancel)?;
                        Ok(Outcome::Text(i18n::tr!(PreferenceSaved).into()))
                    },
                );
                Ok(false)
            }
            Item::AiEnabled => {
                let enabled = !self.config.ai.enabled;
                self.start(
                    item.description(),
                    Effect::Configuration,
                    move |service, cancel, _| {
                        service.configure_with_cancel(ConfigEdit::AiEnabled(enabled), &cancel)?;
                        Ok(Outcome::Text(i18n::tr!(PreferenceSaved).into()))
                    },
                );
                Ok(false)
            }
            Item::Interaction => {
                self.interaction_menu();
                Ok(false)
            }
            Item::Mouse(setting) => {
                let enabled = !self.config.mouse.enabled(setting);
                self.start(
                    item.description(),
                    Effect::Configuration,
                    move |service, _, _| {
                        service.configure(ConfigEdit::Mouse(setting, enabled))?;
                        Ok(Outcome::Text(i18n::tr!(MouseSettingsSaved).into()))
                    },
                );
                Ok(false)
            }
            Item::ChineseCommandTrigger => {
                let enabled = !self.config.chinese_command_trigger;
                // 沿用配置事务和后台保存；成功后再更新开关，失败保留原值并显示错误。
                self.start(
                    item.description(),
                    Effect::Configuration,
                    move |service, _, _| {
                        service.configure(ConfigEdit::ChineseCommandTrigger(enabled))?;
                        Ok(Outcome::Text(i18n::tr!(ChineseCommandTriggerSaved).into()))
                    },
                );
                Ok(false)
            }
        }
    }
}

/// Last rendered focus and geometry, used to reveal changed selection without overriding manual scroll.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Focus {
    pub selected: usize,
    pub area: ratatui::layout::Rect,
}
