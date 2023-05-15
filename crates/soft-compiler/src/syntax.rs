//! This module describes the abstract syntax tree of the soft language and position information.
//! The main structure of this module is the [Expr] that describes a raw s-expression.

use core::fmt;
use std::fmt::Display;

/// A point in a file, a single location inside ir.
#[derive(Debug, Default, Clone, Copy)]
pub struct Point {
    pub line: u64,
    pub column: u64,
}

impl Point {
    /// Advances the point location using a character as the thing that defines if it'll go to other
    /// line or continue at the same.
    pub fn advance(&mut self, char: char) {
        if char == '\n' {
            self.line += 1;
            self.column = 0;
        } else {
            self.column += 1;
        }
    }
}

impl Display for Point {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.line, self.column)
    }
}

/// A range of text in a file.
#[derive(Debug, Default, Clone, Copy)]
pub struct Range {
    pub start: Point,
    pub end: Point,
}

impl Range {
    pub fn new(start: Point, end: Point) -> Self {
        Self { start, end }
    }
}

impl Display for Range {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}-{}", self.start, self.end)
    }
}

/// This Expr represents the concrete syntax tree of the soft language. in the s-expression format.
/// It's used for parsing and will be specialized into a concrete tree in the next step. E.g.:
///
/// ```lisp
/// (print "ata")
/// ```
#[derive(Debug)]
pub enum Expr {
    /// A symbol is a globally available constant that is defined by it's name
    /// that is O(1) for comparison.
    Symbol(Range, String),

    /// An identifier is a name that is used to reference a variable or a function.
    Id(Range, String),

    /// A string literal. It's represented as a UTF-8 array that cannot be indexed.
    Str(Range, String),

    /// An unsigned number literal of 60 bytes.
    Num(Range, u64),

    /// A list is every expression that is surrounded by parenthesis.
    List(Range, Vec<Expr>),
}

impl Expr {
    pub fn is_identifier(&self) -> bool {
        matches!(self, Expr::Id(_, _))
    }
}

impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Expr::Symbol(_, s) => write!(f, ":{s}"),
            Expr::Id(_, s) => write!(f, "{s}"),
            Expr::Str(_, s) => write!(f, "\"{s}\""),
            Expr::Num(_, num) => write!(f, "{num}"),
            Expr::List(_, ls) => write!(
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
