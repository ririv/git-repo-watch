//! Literal multi-path scan arguments, shared by execution and cursor-local completion.
use anyhow::{Result, ensure};
use std::{ops::Range, path::Path};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Quote {
    Bare,
    Single,
    Double,
    EscapeBare,
    EscapeDouble,
}
#[derive(Debug)]
pub(super) struct Token {
    pub range: Range<usize>,
    pub value: String,
    pub state: Quote,
}
pub(super) fn tokens(input: &str) -> Vec<Token> {
    let mut result = Vec::new();
    let mut current: Option<Token> = None;
    for (index, ch) in input.char_indices() {
        if current.is_none() && ch.is_whitespace() {
            continue;
        }
        let token = current.get_or_insert_with(|| Token {
            range: index..index,
            value: String::new(),
            state: Quote::Bare,
        });
        token.range.end = index + ch.len_utf8();
        match (token.state, ch) {
            (Quote::EscapeBare, _) => {
                token.value.push(ch);
                token.state = Quote::Bare;
            }
            (Quote::EscapeDouble, _) => {
                token.value.push(ch);
                token.state = Quote::Double;
            }
            (Quote::Bare, ch) if ch.is_whitespace() => {
                token.range.end = index;
                result.push(current.take().expect("current token"));
            }
            (Quote::Bare, '\'') => token.state = Quote::Single,
            (Quote::Bare, '"') => token.state = Quote::Double,
            (Quote::Single, '\'') | (Quote::Double, '"') => token.state = Quote::Bare,
            (Quote::Bare, '\\') => token.state = Quote::EscapeBare,
            (Quote::Double, '\\') => token.state = Quote::EscapeDouble,
            _ => token.value.push(ch),
        }
    }
    if let Some(token) = current {
        result.push(token);
    }
    result
}
pub(crate) fn request(raw: &str, home: &Path) -> Result<engine::discovery::Request> {
    let tokens = tokens(raw);
    ensure!(
        tokens.iter().all(|token| token.state == Quote::Bare),
        i18n::tr!(UnclosedPathQuote)
    );
    let request = engine::discovery::Request::from_args(
        &tokens
            .into_iter()
            .map(|token| token.value)
            .collect::<Vec<_>>(),
        home,
    )?;
    ensure!(request.root.is_absolute(), i18n::tr!(EnterAnAbsolutePath));
    Ok(request)
}
/// Quotes only when necessary and keeps the cursor inside the token's closing quote.
pub(super) fn encode(value: &str, position: usize, quote: Option<char>) -> (String, usize) {
    if quote == Some('\'') && !value.contains('\'') {
        return (format!("'{value}'"), position + 1);
    }
    if quote.is_none()
        && !value
            .chars()
            .any(|ch| ch.is_whitespace() || matches!(ch, '\'' | '"' | '\\'))
    {
        return (value.into(), position);
    }
    let escape = |text: &str| text.replace('\\', "\\\\").replace('"', "\\\"");
    (
        format!("\"{}\"", escape(value)),
        1 + escape(&value[..position]).len(),
    )
}
