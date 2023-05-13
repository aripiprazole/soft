//! This module describes the abstract syntax tree
//! of the soft language and position information.
//! The main structure of this module is the [Expr]
//! that describes a raw s-expression.

use core::fmt;
use std::fmt::Display;

/// A point in a file, a single location inside ir.
#[derive(Debug, Default, Clone, Copy)]
pub struct Point {
    pub line: u64,
    pub column: u64
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
    pub end: Point
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

#[derive(Debug)]
pub enum Expr {
    Symbol(Range, String),
    Id(Range, String),
    Str(Range, String),
    Num(Range, u64),
    List(Range, Vec<Expr>),
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
