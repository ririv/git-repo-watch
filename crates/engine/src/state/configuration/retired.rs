//! Retire persistent discovery exclusions without changing registered repositories.
use anyhow::{Result, ensure};
use toml_edit::DocumentMut;

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
