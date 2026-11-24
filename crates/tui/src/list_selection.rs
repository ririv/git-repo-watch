//! Shared browsing and multi-selection lifecycle for selectable lists.

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) enum Mode<T> {
    #[default]
    Browsing,
    Selecting(T),
}

impl<T> Mode<T> {
    pub fn active(&self) -> bool {
        matches!(self, Self::Selecting(_))
    }

    pub fn marks(&self) -> Option<&T> {
        match self {
            Self::Browsing => None,
            Self::Selecting(marks) => Some(marks),
        }
    }

    pub fn begin(&mut self) -> &mut T
    where
        T: Default,
    {
        if !self.active() {
            *self = Self::Selecting(T::default());
        }
        let Self::Selecting(marks) = self else {
            unreachable!()
        };
        marks
    }

    pub fn finish(&mut self) {
        *self = Self::Browsing;
    }
}
