//! This module describes the abstract syntax tree of the soft language and position information.
//! The main structure of this module is the [Expr] that describes a raw s-expression.

use core::fmt;

use crate::location::Spanned;

/// An expression represents the concrete syntax tree of the soft language. in the s-expression
/// format.
///
/// It's used for parsing and will be specialized into a concrete tree in the next step. E.g.:
///
/// ```lisp
/// (print "ata")
/// ```
#[derive(Debug, Clone)]
pub enum ExprKind<'a> {
    /// An atom is a globally available constant that is defined by it's name that is O(1) for
    /// comparison.
    ///
    /// E.g:
    /// ```lisp
    /// 'some, 'name, 'some, 'atom
    /// ```
    Atom(&'a str),

    /// An identifier is a name that is used to reference a variable or a function.
    Identifier(&'a str),

    /// An expression list that is surrounded by parenthesis.
    List(Vec<Expr<'a>>),

    /// An unsigned number literal of 60 bytes.
    Number(u64),

    /// A string literal. It's represented as a UTF-8 array that cannot be indexed.
    String(&'a str),
}

/// An [ExprKind] with a range of positions in the source code. It's used in order to make better
/// error messages.
pub type Expr<'a> = Spanned<ExprKind<'a>>;

impl<'a> Expr<'a> {
    pub fn is_identifier(&self) -> bool {
        matches!(self.data, ExprKind::Identifier(_))
    }

    /// This function gets the ownership of the reference without copying it entirely (probably just
    /// a shallow copy to the stack) and changes it's value to an empty list.
    pub fn take(&mut self) -> Expr<'a> {
        let mut result = Spanned {
            data: ExprKind::List(vec![]),
            loc: self.loc.clone(),
        };

        std::mem::swap(self, &mut result);

        result
    }

    /// Function that makes it easier to get a name from an Expr in order to use a `bind` operation
    /// on it.
    pub fn get_identifier(&self) -> Option<&'a str> {
        match self.data {
            ExprKind::Identifier(str) => Some(str),
            _ => None,
        }
    }

    pub fn get_list(&mut self) -> Option<&mut [Expr<'a>]> {
        match self.data {
            ExprKind::List(ref mut ls) => Some(ls),
            _ => None,
        }
    }
}

impl<'a> fmt::Display for ExprKind<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExprKind::Atom(s) => write!(f, ":{s}"),
            ExprKind::Identifier(s) => write!(f, "{s}"),
            ExprKind::String(s) => write!(f, "\"{s}\""),
            ExprKind::Number(num) => write!(f, "{num}"),
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
        self.data.fmt(f)
    }
}
