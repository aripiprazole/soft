//! This module describes a [Loc] that is a newtype wrapper that is used to localize things inside
//! the source code. It's used to represent the character representation in bytes.

use std::{
    fmt::Debug,
    ops::{AddAssign, Range},
};

/// Byte address of a character in the source code.
#[derive(Debug, Clone, Copy, Default)]
pub struct Loc(pub usize);

/// Data localized in the code with [Range<Loc>]. The reason to store the location span, is to
/// have debug and error handling facilities.
#[derive(Debug, Clone)]
pub struct Spanned<T> {
    pub data: T,
    pub loc: Range<Loc>,
}

impl<T> Spanned<T> {
    pub fn stub(data: T) -> Self {
        Self {
            data,
            loc: Loc(0)..Loc(0),
        }
    }

    pub fn new(loc: Range<Loc>, data: T) -> Self {
        Self { data, loc }
    }

    pub fn map<U>(&self, fun: fn(&T) -> U) -> Spanned<U> {
        Spanned {
            data: fun(&self.data),
            loc: self.loc.clone(),
        }
    }

    pub fn with<U>(&self, data: U) -> Spanned<U> {
        Spanned {
            data,
            loc: self.loc.clone(),
        }
    }
}

impl AddAssign for Loc {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
    }
}

impl From<Loc> for usize {
    fn from(loc: Loc) -> Self {
        loc.0
    }
}

impl<T> From<T> for Spanned<T> {
    fn from(value: T) -> Self {
        Spanned::new(Loc(0)..Loc(0), value)
    }
}
