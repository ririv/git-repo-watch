//! Bounded JSON-lines framing independent of agent event schemas.
use anyhow::{Context, Result, ensure};
use serde_json::Value;
use std::io::{BufRead, Read, Write};

pub(super) struct Reader<R> {
    input: R,
    remaining: usize,
}
impl<R: BufRead> Reader<R> {
    pub fn new(input: R) -> Self {
        Self {
            input,
            remaining: 8 * 1024 * 1024,
        }
    }
    pub fn next(&mut self) -> Result<Value> {
        loop {
            let mut line = String::new();
            let count = self
                .input
                .by_ref()
                .take(self.remaining.min(1024 * 1024) as u64 + 1)
                .read_line(&mut line)?;
            ensure!(count > 0, i18n::tr!(AgentStreamIncomplete));
            ensure!(
                count <= self.remaining && count <= 1024 * 1024,
                i18n::tr!(AgentOutputTooLarge)
            );
            self.remaining -= count;
            if line.trim().is_empty() {
                continue;
            }
            return serde_json::from_str(&line).context(i18n::tr!(AgentMessageInvalid));
        }
    }
}
pub(super) fn send(output: &mut impl Write, value: Value) -> Result<()> {
    serde_json::to_writer(&mut *output, &value)?;
    output.write_all(b"\n")?;
    output.flush()?;
    Ok(())
}
pub(super) fn text(value: &Value) -> Result<&str> {
    value.as_str().context(i18n::tr!(AgentMessageInvalid))
}
