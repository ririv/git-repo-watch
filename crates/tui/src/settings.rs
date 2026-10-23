//! Settings-page entries can open command editors or update preferences directly.
use crate::{Action, Configuration, Effect, Outcome, Screen, command};
use anyhow::Result;
use engine::{cancel::Cancel, scheduler::Registration, service::ConfigEdit};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Item {
    Command(Action),
    ChineseCommandTrigger,
}
impl Item {
    pub fn description(self) -> &'static str {
        match self {
            Self::Command(action) => command::description(action),
            Self::ChineseCommandTrigger => i18n::tr!(ChineseCommandTrigger),
        }
    }
}
impl Configuration {
    pub(crate) fn items(self, registration: Registration) -> Vec<Item> {
        let mut items = self
            .actions(registration)
            .iter()
            .copied()
            .map(Item::Command)
            .collect::<Vec<_>>();
        if self == Self::General && i18n::language() == i18n::Language::Chinese {
            items.push(Item::ChineseCommandTrigger);
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
