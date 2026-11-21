//! Give the editor access to the current Windows console.
use crate::editor::Plan;
use anyhow::{Context, Result, ensure};
use std::process::{Command, Stdio};

pub fn run(plan: &Plan) -> Result<()> {
    let status = Command::new(&plan.program)
        .args(&plan.arguments)
        .current_dir(&plan.directory)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .with_context(|| i18n::tf!(UnableToStart, plan.program.display()))?;
    ensure!(
        status.success(),
        "{}: {status}",
        i18n::tr!(ExternalEditorFailed)
    );
    Ok(())
}
