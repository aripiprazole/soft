//! This module describes a [Loc] that is a new-type wrapper that is used to localize things inside
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
    pub range: Range<Loc>,
}

impl<T> Spanned<T> {
    /// Creates a new spanned data with the given location and data.
    pub fn new(loc: Range<Loc>, data: T) -> Self {
        Self { data, range: loc }
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
