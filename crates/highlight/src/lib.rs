//! Bounded source highlighting with complete lexical context and no UI dependencies.
mod source;
#[cfg(test)]
mod tests;
pub use source::{Highlighted, Limit, Source, Token, highlight};
use std::sync::LazyLock;
use syntect::{highlighting::ThemeSet, parsing::SyntaxSet};

static SYNTAXES: LazyLock<SyntaxSet> = LazyLock::new(SyntaxSet::load_defaults_newlines);
static THEMES: LazyLock<ThemeSet> = LazyLock::new(ThemeSet::load_defaults);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color(pub u8, pub u8, pub u8);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Style {
    pub foreground: Color,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
}
