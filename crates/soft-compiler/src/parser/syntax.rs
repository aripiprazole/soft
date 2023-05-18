//! This module describes the abstract syntax tree of the soft language and position information.
//! The main structure of this module is the [Expr] that describes a raw s-expression.

use core::fmt;
use std::ops::Range;

use crate::location::Loc;

/// This Expr represents the concrete syntax tree of the soft language. in the s-expression format.
/// It's used for parsing and will be specialized into a concrete tree in the next step. E.g.:
///
/// ```lisp
/// (print "ata")
/// ```
#[derive(Debug, Clone)]
pub enum ExprKind<'a> {
    /// A symbol is a globally available constant that is defined by it's name
    /// that is O(1) for comparison.
    Symbol(&'a str),

    /// An identifier is a name that is used to reference a variable or a function.
    Id(&'a str),

    /// A string literal. It's represented as a UTF-8 array that cannot be indexed.
    Str(&'a str),

    /// An unsigned number literal of 60 bytes.
    Num(u64),

    /// A list is every expression that is surrounded by parenthesis.
    List(Vec<Expr<'a>>),
}

/// A ExprKind with a range of positions in the source code. It's used in order to make better error
/// messages.
#[derive(Debug, Clone)]
pub struct Expr<'a> {
    pub kind: ExprKind<'a>,
    pub loc: Range<Loc>,
}

impl<'a> Expr<'a> {
    pub fn new(kind: ExprKind<'a>, loc: Range<Loc>) -> Self {
        Self { kind, loc }
    }

    pub fn is_identifier(&self) -> bool {
        matches!(self.kind, ExprKind::Id(_))
    }
}

impl<'a> fmt::Display for ExprKind<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExprKind::Symbol(s) => write!(f, ":{s}"),
            ExprKind::Id(s) => write!(f, "{s}"),
            ExprKind::Str(s) => write!(f, "\"{s}\""),
            ExprKind::Num(num) => write!(f, "{num}"),
            ExprKind::List(ls) => write!(
                f,
                "({})",
                ls.iter()
                    .map(|e| e.to_string())
                    .collect::<Vec<_>>()
                    .join(" ")
            ),
        }
    }
}

impl<'a> fmt::Display for Expr<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.kind.fmt(f)
    }
}
