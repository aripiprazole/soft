//! This module describes a [Loc] that is a newtype wrapper that is used to localize things inside
//! the source code. It's used to represent the character representation in bytes.

use std::ops::AddAssign;

/// Byte address of a character in the source code.
#[derive(Debug, Clone, Copy)]
pub struct Loc(pub usize);

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
