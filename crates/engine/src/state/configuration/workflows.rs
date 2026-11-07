//! Explicit conversion of the former scheduled push policy into an ordered workflow.
use crate::{domain::PushPolicy, workflow::Plan};
use anyhow::{Context, Result, ensure};
use toml_edit::{DocumentMut, Item, Value};

pub(super) fn migrate(document: &mut DocumentMut) -> Result<()> {
    let Some(legacy) = document.get("push_policy") else {
        return Ok(());
    };
    ensure!(
        !document.contains_key("scheduled_actions"),
        i18n::tr!(WorkflowLegacyConflict)
    );
    let policy: PushPolicy = legacy
        .as_str()
        .context(i18n::tr!(WorkflowPolicyString))?
        .parse()?;
    let mut value = Value::Array(
        Plan::from_policy(policy)
            .steps()
            .iter()
            .map(|step| Value::from(step.token()))
            .collect(),
    );
    *value.decor_mut() = legacy
        .as_value()
        .context(i18n::tr!(WorkflowPolicyString))?
        .decor()
        .clone();
    let mut key_decor = document
        .key("push_policy")
        .expect("legacy key")
        .leaf_decor()
        .clone();
    if let Some(prefix) = key_decor.prefix().and_then(|prefix| prefix.as_str()) {
        let prefix = prefix.replace(
            "# 定时任务的自动推送策略：disabled 或 fast_forward。",
            "# 定时流程：按数组顺序执行，最后 inspect 读取并保存状态。",
        );
        key_decor.set_prefix(prefix);
    }
    document.remove("push_policy");
    document.insert("scheduled_actions", Item::Value(value));
    *document
        .key_mut("scheduled_actions")
        .expect("converted key")
        .leaf_decor_mut() = key_decor;
    Ok(())
}
pub(super) fn migrate_json(value: &mut serde_json::Value) -> Result<()> {
    let object = value
        .as_object_mut()
        .context(i18n::tr!(WorkflowObjectExpected))?;
    let Some(policy) = object.remove("push_policy") else {
        return Ok(());
    };
    ensure!(
        !object.contains_key("scheduled_actions"),
        i18n::tr!(WorkflowLegacyConflict)
    );
    let policy: PushPolicy = serde_json::from_value(policy)?;
    object.insert(
        "scheduled_actions".into(),
        serde_json::to_value(Plan::from_policy(policy))?,
    );
    Ok(())
}
