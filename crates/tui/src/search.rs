//! Search semantics and match navigation are independent of list rendering.
mod completion;
pub(crate) mod highlight;
mod literal;
mod screen;
#[cfg(test)]
mod tests;
pub(crate) mod text;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Mode {
    #[default]
    Filter,
    Locate,
}
impl Mode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Filter => i18n::tr!(SearchFilter),
            Self::Locate => i18n::tr!(SearchLocate),
        }
    }
    pub fn includes(self, matched: bool) -> bool {
        self == Self::Locate || matched
    }
}

/// A view exposes only the search modes that preserve its meaning.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Support {
    Unavailable,
    Filter,
    Locate,
    Both(Mode),
}
impl Support {
    pub fn mode(self) -> Option<Mode> {
        match self {
            Self::Unavailable => None,
            Self::Filter => Some(Mode::Filter),
            Self::Locate => Some(Mode::Locate),
            Self::Both(mode) => Some(mode),
        }
    }
    pub fn admits(self, action: crate::Action) -> bool {
        match action {
            crate::Action::SearchMenu => self != Self::Unavailable,
            crate::Action::Search(mode) => match self {
                Self::Unavailable => false,
                Self::Filter => mode == Mode::Filter,
                Self::Locate => mode == Mode::Locate,
                Self::Both(_) => true,
            },
            _ => true,
        }
    }
}

#[derive(Debug)]
pub(crate) struct Matches(Vec<usize>);
impl Matches {
    pub fn new(rows: impl IntoIterator<Item = bool>) -> Self {
        Self(
            rows.into_iter()
                .enumerate()
                .filter_map(|(i, matched)| matched.then_some(i))
                .collect(),
        )
    }
    pub fn nearest(&self, current: Option<usize>) -> Option<usize> {
        self.0
            .iter()
            .copied()
            .find(|&i| i >= current.unwrap_or(0))
            .or_else(|| self.0.first().copied())
    }
    pub fn step(&self, current: Option<usize>, backwards: bool) -> Option<usize> {
        if backwards {
            self.0
                .iter()
                .rev()
                .copied()
                .find(|&i| current.is_some_and(|current| i < current))
                .or_else(|| self.0.last().copied())
        } else {
            self.0
                .iter()
                .copied()
                .find(|&i| current.is_none_or(|current| i > current))
                .or_else(|| self.0.first().copied())
        }
    }
    pub fn label(&self, current: Option<usize>) -> String {
        match self.0.iter().position(|&i| Some(i) == current) {
            Some(index) => format!("{}/{}", index + 1, self.0.len()),
            None if self.0.is_empty() => "0/0".into(),
            None => format!("—/{}", self.0.len()),
        }
    }
}
