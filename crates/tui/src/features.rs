//! Semantic maturity shared by navigation and command presentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Maturity {
    Stable,
    Experimental,
}
