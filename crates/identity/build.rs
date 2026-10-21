//! Validate shared shell-compatible application identity and emit compile-time constants.
use std::{collections::BTreeMap, env, fs, path::PathBuf};

fn main() {
    let source = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("manifest directory"))
        .join("../../application.env");
    println!("cargo:rerun-if-changed={}", source.display());
    let text = fs::read_to_string(source).expect("read application.env");
    let fields = [
        ("APP_COMMAND", "COMMAND", "command"),
        ("APP_DISPLAY_NAME", "DISPLAY_NAME", "display_name"),
        ("APP_DATA_DIRECTORY", "DATA_DIRECTORY", "data_directory"),
        ("APP_SERVICE_LABEL", "SERVICE_LABEL", "service_label"),
    ];
    let mut values = BTreeMap::new();
    for line in text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
    {
        let (key, value) = line
            .split_once('=')
            .expect("expected KEY=VALUE in application.env");
        assert!(
            fields.iter().any(|(field, _, _)| *field == key),
            "unknown identity field: {key}"
        );
        assert!(
            !value.is_empty()
                && value
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || b"._-".contains(&c)),
            "identity values must be shell-safe ASCII identifiers"
        );
        assert!(
            value != "." && value != "..",
            "identity value must not be a directory traversal component"
        );
        assert!(
            values.insert(key, value).is_none(),
            "duplicate identity field: {key}"
        );
    }
    let command = values.get("APP_COMMAND").expect("APP_COMMAND is required");
    assert!(
        command.as_bytes()[0].is_ascii_alphabetic(),
        "APP_COMMAND must start with a letter"
    );
    let mut generated = String::from("// Generated from application.env; do not edit.\n");
    for (key, constant, macro_name) in fields {
        let value = values
            .get(key)
            .unwrap_or_else(|| panic!("{key} is required"));
        generated.push_str(&format!("pub const {constant}: &str = {value:?};\n"));
        generated.push_str(&format!(
            "#[macro_export]\nmacro_rules! {macro_name} {{ () => {{ {value:?} }}; }}\n"
        ));
    }
    let output = PathBuf::from(env::var_os("OUT_DIR").expect("build output directory"));
    fs::write(output.join("identity.rs"), generated).expect("write generated identity");
}
