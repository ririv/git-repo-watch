//! Explicitly retire obsolete configuration fields while preserving user comments.
use anyhow::{Result, ensure};
use toml_edit::DocumentMut;

pub(super) fn pending(document: &DocumentMut) -> bool {
    ["excluded_roots", "check_on_startup", "startup_actions"]
        .iter()
        .any(|key| document.contains_key(key))
}

pub(super) fn startup(document: &mut DocumentMut) -> Result<()> {
    if let Some(value) = document.get("check_on_startup") {
        ensure!(value.as_bool().is_some(), i18n::tr!(InvalidToml));
    }
    if let Some(value) = document.get("startup_actions") {
        let steps = value
            .as_array()
            .ok_or_else(|| anyhow::anyhow!(i18n::tr!(InvalidToml)))?;
        let tokens = steps
            .iter()
            .map(|step| {
                step.as_str()
                    .ok_or_else(|| anyhow::anyhow!(i18n::tr!(InvalidToml)))
            })
            .collect::<Result<Vec<_>>>()?;
        tokens.join(" ").parse::<crate::workflow::Plan>()?;
    }
    for (key, defaults) in [
        (
            "check_on_startup",
            &[
                "# 启动 TUI 时是否对全部仓库执行 startup_actions。",
                "# 启动 TUI 时是否后台检查全部注册仓库。",
                "# 启动 TUI 时是否在后台检查全部仓库。",
            ] as &[_],
        ),
        ("startup_actions", &["# 启动流程：按数组顺序执行。"] as &[_]),
    ] {
        remove(document, key, defaults);
    }
    Ok(())
}

fn remove(document: &mut DocumentMut, key: &str, defaults: &[&str]) {
    let Some(value) = document.get(key) else {
        return;
    };
    let suffix = value
        .as_value()
        .and_then(|value| value.decor().suffix())
        .and_then(|value| value.as_str())
        .unwrap_or("");
    let mut comment = document
        .key(key)
        .expect("existing key")
        .leaf_decor()
        .prefix()
        .and_then(|value| value.as_str())
        .unwrap_or("")
        .to_owned();
    for default in defaults {
        comment = comment.replace(&format!("{default}\n"), "");
    }
    if suffix.contains('#') {
        comment.push_str(suffix.trim_start());
        comment.push('\n');
    }
    document.remove(key);
    let first = document.iter().next().map(|(key, _)| key.to_owned());
    if !comment.trim().is_empty()
        && let Some(first) = first
    {
        let mut key = document.key_mut(&first).expect("first key");
        let decor = key.leaf_decor_mut();
        let current = decor
            .prefix()
            .and_then(|value| value.as_str())
            .unwrap_or("");
        decor.set_prefix(format!("{comment}{current}"));
    }
}

pub(super) fn json_startup(value: &mut serde_json::Value) -> Result<()> {
    let object = value
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!(i18n::tr!(InvalidJson)))?;
    if let Some(value) = object.remove("check_on_startup") {
        ensure!(value.is_boolean(), i18n::tr!(InvalidJson));
    }
    if let Some(value) = object.remove("startup_actions") {
        serde_json::from_value::<crate::workflow::Plan>(value)?;
    }
    Ok(())
}

pub(super) fn exclusions(document: &mut DocumentMut) -> Result<()> {
    let Some(value) = document.get("excluded_roots") else {
        return Ok(());
    };
    ensure!(
        value
            .as_array()
            .is_some_and(|array| array.iter().all(|value| value.as_str().is_some())),
        i18n::tr!(InvalidToml)
    );
    let prefix = document
        .key("excluded_roots")
        .expect("existing key")
        .leaf_decor()
        .prefix()
        .and_then(|value| value.as_str())
        .unwrap_or("")
        .replace("# 扫描时排除的目录绝对路径。\n", "");
    document.remove("excluded_roots");
    // 退役字段自己的默认说明一并移除，用户的独立注释保留在配置开头。
    let first = document.iter().next().map(|(key, _)| key.to_owned());
    if !prefix.trim().is_empty()
        && let Some(first) = first
    {
        let mut key = document.key_mut(&first).expect("first key");
        let decor = key.leaf_decor_mut();
        let current = decor
            .prefix()
            .and_then(|value| value.as_str())
            .unwrap_or("");
        let combined = format!("{prefix}{current}");
        decor.set_prefix(combined);
    }
    Ok(())
}
pub(super) fn json_exclusions(value: &mut serde_json::Value) -> Result<()> {
    if let Some(object) = value.as_object_mut()
        && let Some(value) = object.remove("excluded_roots")
    {
        ensure!(
            value
                .as_array()
                .is_some_and(|array| array.iter().all(|value| value.is_string())),
            i18n::tr!(InvalidJson)
        );
    }
    Ok(())
}
