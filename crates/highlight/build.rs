//! Compile bundled grammars ahead of time; runtime highlighting needs no YAML loader.
use std::{env, error::Error, fs, path::PathBuf};
use syntect::{dumps::dump_to_uncompressed_file, parsing::SyntaxDefinition};

fn main() -> Result<(), Box<dyn Error>> {
    let base = two_face::syntax::extra_newlines();
    let existing_unlinked = base.find_unlinked_contexts();
    let mut builder = base.into_builder();
    for filename in [
        "JSONC.sublime-syntax",
        "JSON5.sublime-syntax",
        "Astro.sublime-syntax",
    ] {
        let path = PathBuf::from("syntaxes").join(filename);
        println!("cargo:rerun-if-changed={}", path.display());
        let definition = SyntaxDefinition::load_from_str(&fs::read_to_string(&path)?, true, None)
            .map_err(|error| format!("{}: {error}", path.display()))?;
        builder.add(definition);
    }
    let set = builder.build();
    // 上游语法包已有的未链接引用不在此处修补；新增定义必须完整链接。
    let unlinked = set.find_unlinked_contexts();
    let added: Vec<_> = unlinked.difference(&existing_unlinked).collect();
    if !added.is_empty() {
        return Err(format!("Bundled grammars contain unresolved contexts: {added:?}").into());
    }
    let output = PathBuf::from(env::var_os("OUT_DIR").ok_or("OUT_DIR is missing")?);
    dump_to_uncompressed_file(&set, output.join("syntaxes.packdump"))?;
    Ok(())
}
