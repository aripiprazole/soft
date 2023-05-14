//! This module describes the abstract syntax tree of the soft language and position information.
//! The main structure of this module is the [Expr] that describes a raw s-expression.

use core::fmt;
use std::fmt::Display;

/// A point in a file, a single location inside ir.
#[derive(Debug, Default, Clone, Copy)]
pub struct Point {
    pub line: u64,
    pub column: u64
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

/// This Expr represents the concrete syntax tree of the soft language. in the s-expression format.
/// It's used for parsing and will be specialized into a concrete tree in the next step. E.g.:
///
/// ```lisp
/// (print "ata")
/// ```
#[derive(Debug)]
pub enum Expr<'a> {
    /// A symbol is a globally available constant that is defined by it's name
    /// that is O(1) for comparison.
    Symbol(Range, &'a str),

    /// An identifier is a name that is used to reference a variable or a function.
    Id(Range, &'a str),

    /// A string literal. It's represented as a UTF-8 array that cannot be indexed.
    Str(Range, String),

    /// An unsigned number literal of 60 bytes.
    Num(Range, u64),

    /// A list is every expression that is surrounded by parenthesis.
    List(Range, Vec<Expr<'a>>),
}

impl<'a> fmt::Display for Expr<'a> {
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

macro_rules! sexpr_match_list {
    (($($out:tt)*) => @ $name:ident ($($t:tt)*) $($rest:tt)*) => {
        sexpr_match_list!(
            $($out)*, $crate::syntax::Expr::List(_, $name) =>
            $($rest)*
        )
    };
    (($($out:tt)*) => <$s:literal> $($rest:tt)*) => {
        sexpr_match_list!(
            $($out)*, $crate::syntax::Expr::Id(_, $name) =>
            $($rest)*
        )
    };
    (($($out:tt)*) => <$s:ident:expr> $($rest:tt)*) => {
        sexpr_match_list!(
            $($out)*, $s =>
            $($rest)*
        )
    };
    (($($out:tt)*) => $s:literal $($rest:tt)*) => {
        sexpr_match_list!(
            $($out)* $crate::syntax::Expr::Id(_, $s) =>
            $($rest)*
        )
    };
    ( $out:pat => $($rest:tt)*) => { $out };
}

macro_rules! sexpr_match_pat {
    (@ $name:ident ($($t:tt)*) ) => {
        $crate::syntax::Expr::List(_, $name)
    };
    (<$s:literal>) => {
        $crate::syntax::Expr::Id(_, $s)
    };
    (<$s:ident:expr>) => {
        $s
    };
    ($s:literal) => {
        $crate::syntax::Expr::Id(_, $s)
    };
}

macro_rules! sexpr_match_expr {
    ($expr:expr, <$s:literal>) => {$expr};
    ($expr:expr, <$s:ident:expr>) => {$expr};
    ($expr:expr, $s:literal) => {$expr};
    ($expr:expr, @$name:ident($($t:tt)*)) => {

        match $name.as_slice() {
            [
                sexpr_match_list!(() => $($t)*)] => $expr,
            _ => todo!()
        }
    }
}

macro_rules! sexpr_match {
    ($name:expr, $($clause:tt => $expr:expr),*) => {
        match $name {
            $(sexpr_match_pat!($clause) => sexpr_match_expr!($clause, $expr),)*
            _ => panic!("pudim")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Expr;

    #[test]
    pub fn test() {
        let t = Expr::List(Default::default(), vec![
            Expr::Id(Default::default(), "set!"),
            Expr::Id(Default::default(), "a"),
            Expr::Num(Default::default(), 3),
        ]);

        let r = match todo!() {
            sexpr_match_pat!(@l("set!" <a>)) => sexpr_match_expr!(2, @l("set!" <a>))
        };
    }
}