#![feature(box_patterns)]
#![feature(try_trait_v2)]

use std::fmt::Display;

pub type Result<T, E = SemanticError> = std::result::Result<T, E>;

/// Evaluation of the Soft programming language, it will transform an [crate::Expr] into
/// a [crate::eval::Value].
pub mod eval;

/// Tokenization/lexing and parsing of Soft programming language, it will transform a string
/// into [crate::Term].
pub mod parser;

/// Term is a recursive data structure that represents a list of terms, an atom, an identifier,
/// or an integer.
///
/// It's the first part of our Abstract-Syntax-Tree (AST).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Term {
    List(Vec<Term>),    // (a b c)
    Vec(Vec<Term>),     // [a b c]
    Atom(String),       // :bla
    Identifier(String), // bla
    Int(u64),           // 123
    Float(u64, u64),    // 123.456
    String(String),     // "some stuff"
    SrcPos(SrcPos, Box<Term>),
}

pub trait ExprKind: Sized {
    fn try_new(term: Term) -> Result<Option<Expr>>;
}

// Abstract-Syntax-Tree (AST) for the Soft programming language. It's the representation
// of abstract terms in the language.
//
// It's the specialized phase of the language, where we have a concrete syntax and we
// can start to evaluate the program.
define_ast!(Expr, {
    Fun,      // (fun* [a b] (+ a b))
    List,     // [a b c] or (list a b c)
    Apply,    // (a b c) or (apply a b c)
    Def,      // (def* a 123)
    Recur,    // (recur a)
    DefMacro, // (defmacro* a (fun (a b) (+ a b))
    Quote,    // '(fun* (a b) (+ a b))
    Literal   // 123 | "bla" | :bla | bla
});

define_builtin!(DefMacro, "defmacro*", 2);
define_builtin!(Def, "def*", 2);
define_builtin!(Recur, "recur");
define_builtin!(Fun, "fun*", 3);
define_builtin!(Quote, "quote", 2);
define_builtin!(Apply, "apply");

/// Semantic errors that can occur during the specialization of an expression.
#[derive(thiserror::Error, Debug, Clone)]
pub enum SemanticError {
    #[error("invalid expression")]
    InvalidExpression,

    #[error("failed to match equivalent expression")]
    FailedToMatch,

    #[error("invalid list")]
    InvalidList,

    #[error("invalid arguments")]
    InvalidArguments,

    #[error("missing function parameters")]
    MissingParameters,

    #[error("missing function body")]
    MissingBody,

    #[error("missing application head")]
    MissingHead,

    #[error("expected string")]
    ExpectedString,

    #[error("expected vector with size {0}")]
    ExpectedVectorWithSize(usize),

    #[error("invalid quote expression")]
    ExpectedQuoteExpression,
}

/// Meta information about a term, or any other part of the AST.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SrcPos {
    pub byte: std::ops::Range<usize>,
    pub file: String,
}

impl SrcPos {
    pub fn next(&mut self, c: char) {
        self.byte.end += c.len_utf8();
    }

    pub fn reset(&mut self) {
        self.byte.start = self.byte.end;
    }
}

impl From<SemanticError> for Expr {
    fn from(value: SemanticError) -> Self {
        match value {
            SemanticError::InvalidExpression => keyword!("error/invalid-expression"),
            SemanticError::FailedToMatch => keyword!("error/failed-to-match"),
            SemanticError::InvalidList => keyword!("error/invalid-list"),
            SemanticError::InvalidArguments => keyword!("error/invalid-arguments"),
            SemanticError::MissingParameters => keyword!("error/missing-parameters"),
            SemanticError::MissingBody => keyword!("error/missing-body"),
            SemanticError::MissingHead => keyword!("error/missing-head"),
            SemanticError::ExpectedString => keyword!("error/expected-string"),
            SemanticError::ExpectedVectorWithSize(size) => {
                soft_vec![keyword!("error/expected-vector"), size]
            }
            SemanticError::ExpectedQuoteExpression => {
                keyword!("error/expected-quote-expression")
            }
        }
    }
}

impl Term {
    fn width(&self) -> usize {
        match self {
            Term::List(s) | Term::Vec(s) => {
                let mut width = 2;
                for t in s {
                    width += t.width();
                }
                width
            }
            Term::Atom(s) => s.len(),
            Term::Identifier(s) => s.len(),
            Term::Int(n) => n.to_string().len(),
            Term::Float(n, u) => n.to_string().len() + u.to_string().len() + 1,
            Term::String(s) => s.len(),
            Term::SrcPos(_, t) => t.width(),
        }
    }

    fn pretty_print(&self, f: &mut std::fmt::Formatter<'_>, indent: usize) -> std::fmt::Result {
        match self {
            Term::List(s) | Term::Vec(s) => {
                if self.width() + indent > 80 {
                    write!(f, "{:indent$}(", "", indent = indent)?;
                    for t in s {
                        write!(f, "{:indent$}", "", indent = indent + 2)?;
                        t.pretty_print(f, indent + 2)?;
                        writeln!(f)?;
                    }
                    write!(f, "{:indent$})", "", indent = indent)?;
                }

                Ok(())
            }
            Term::Atom(s) => write!(f, ":{}", s),
            Term::Identifier(s) => write!(f, "{}", s),
            Term::Int(s) => write!(f, "{}", s),
            Term::Float(u, n) => write!(f, "{}.{}", u, n),
            Term::String(s) => write!(f, "\"{}\"", s),
            Term::SrcPos(_, t) => t.pretty_print(f, indent),
        }
    }

    pub fn transport(self, with: Term) -> Term {
        if let Term::SrcPos(src_pos, _) = self {
            Term::SrcPos(src_pos, with.into())
        } else {
            with
        }
    }

    pub fn at(&self, nth: usize) -> Option<Term> {
        if let Term::List(ls) = self {
            ls.get(nth).cloned()
        } else {
            None
        }
    }

    pub fn split(&self) -> Option<(Term, Vec<Term>)> {
        if let Term::List(ls) = self {
            let (first, rest) = ls.split_first()?;
            Some((first.clone(), rest.to_vec()))
        } else {
            None
        }
    }

    pub fn is_keyword(&self, keyword: &str) -> bool {
        matches!(self, Term::Identifier(x) if x == keyword)
    }

    pub fn spine(&self) -> Option<Vec<Term>> {
        if let Term::List(ls) = self.clone() {
            Some(ls.clone())
        } else {
            None
        }
    }

    /// Removes meta information from a term.
    pub fn unbox(self) -> Term {
        match self {
            Term::SrcPos(_, t) => t.unbox(),
            Term::List(x) => Term::List(x.into_iter().map(|t| t.unbox()).collect()),
            t => t,
        }
    }
}

/// Fun expression constructs, it's a function that has a list of parameters and a body.
pub mod fun {
    use super::*;

    impl Fun {
        /// Returns the name of the function.
        pub fn name(&self) -> Result<Expr> {
            self.0
                .at(1)
                .ok_or(SemanticError::InvalidExpression)?
                .try_into()
        }

        /// Returns a list of parameters that are in the spine of the function.
        pub fn parameters(&self) -> Result<List> {
            self.0
                .at(2)
                .map(List)
                .ok_or(SemanticError::MissingParameters)
        }

        /// Returns the body of the function.
        pub fn body(&self) -> Result<Expr> {
            self.0.at(3).ok_or(SemanticError::MissingBody)?.try_into()
        }
    }
}

/// List expression construct, it's a list of expressions.
pub mod list {
    use SemanticError::*;

    use super::*;

    impl ExprKind for List {
        fn try_new(term: Term) -> Result<Option<Expr>> {
            if let Term::Vec(ref vec) | Term::SrcPos(_, box Term::Vec(ref vec)) = term {
                let items = vec.clone().into();
                return Ok(Some(List(term.transport(items)).into()));
            }

            let (head, tail) = term.split().ok_or(InvalidExpression)?;
            if head.is_keyword("list") {
                let tail = assert_length(tail, 1)?;
                Ok(Some(List(term.transport(tail.into())).into()))
            } else {
                Ok(None)
            }
        }
    }

    impl List {
        /// Returns a list of expressions that are in the spine of the list.
        pub fn elements(&self) -> Result<Vec<Expr>> {
            self.0
                .spine()
                .ok_or(SemanticError::InvalidList)?
                .into_iter()
                .map(Expr::try_from)
                .collect()
        }
    }
}

/// Apply expression construct, it's a function application.
pub mod apply {
    use super::*;

    impl Apply {
        /// Returns the callee of the application.
        pub fn callee(&self) -> Result<Expr> {
            self.0.at(0).ok_or(SemanticError::MissingHead)?.try_into()
        }

        /// Returns a list of arguments that are in the spine of the application.
        pub fn spine(&self) -> Result<Vec<Expr>> {
            self.0
                .spine()
                .ok_or(SemanticError::InvalidArguments)?
                .into_iter()
                .skip(1) // Skip the head of the application.
                .map(Expr::try_from)
                .collect()
        }
    }
}

/// Recur expression construct, it's a function application.
pub mod recur {
    use super::*;

    impl Recur {
        /// Returns a list of arguments that are in the spine of the application.
        pub fn spine(&self) -> Result<Vec<Expr>> {
            self.0
                .spine()
                .ok_or(SemanticError::InvalidArguments)?
                .into_iter()
                .skip(1) // Skip the head of the application.
                .map(Expr::try_from)
                .collect()
        }
    }
}

/// Define expression construct, it's a definition of a value.
pub mod def {
    pub use super::*;

    impl Def {
        /// Returns the name of the definition.
        pub fn name(&self) -> Result<Expr> {
            self.0
                .at(1)
                .ok_or(SemanticError::InvalidExpression)?
                .try_into()
        }

        /// Returns the value of the definition.
        pub fn value(&self) -> Result<Expr> {
            self.0
                .at(2)
                .ok_or(SemanticError::InvalidExpression)?
                .try_into()
        }
    }
}

/// Macro define expression construct, it's a definition of a value.
pub mod defmacro {
    pub use super::*;

    impl DefMacro {
        /// Returns the name of the definition.
        pub fn name(&self) -> Result<Expr> {
            self.0
                .at(1)
                .ok_or(SemanticError::InvalidExpression)?
                .try_into()
        }

        /// Returns the value of the definition.
        pub fn value(&self) -> Result<Expr> {
            self.0
                .at(2)
                .ok_or(SemanticError::InvalidExpression)?
                .try_into()
        }
    }
}

/// Quote expression construct, it's a quoted expression.
pub mod quote {
    use super::*;

    impl Quote {
        /// Returns the quoted expression.
        pub fn expr(&self) -> Result<Expr> {
            self.0
                .at(1)
                .ok_or(SemanticError::InvalidExpression)?
                .try_into()
        }
    }
}

/// Literal expression construct, it's a literal value.
pub mod literal {
    use super::*;

    impl ExprKind for Literal {
        fn try_new(term: Term) -> Result<Option<Expr>> {
            Ok(Some(Expr::Literal(Literal(term))))
        }
    }

    impl From<String> for Expr {
        fn from(value: String) -> Self {
            Expr::Literal(Literal(Term::String(value)))
        }
    }

    impl From<u64> for Expr {
        fn from(value: u64) -> Self {
            Expr::Literal(Literal(Term::Int(value)))
        }
    }

    impl From<usize> for Expr {
        fn from(value: usize) -> Self {
            Expr::Literal(Literal(Term::Int(value as u64)))
        }
    }

    impl Expr {
        pub fn new_keyword(keyword: &str) -> Self {
            Expr::Literal(Literal(Term::Atom(keyword.to_string())))
        }

        /// Expects a string literal and returns it's value.
        pub fn string(&self) -> Result<String> {
            match self {
                Expr::Literal(Literal(Term::String(string))) => Ok(string.clone()),
                _ => Err(SemanticError::ExpectedString),
            }
        }
    }
}

fn assert_length(list: Vec<Term>, length: usize) -> Result<Vec<Term>, SemanticError> {
    if list.len() != length {
        Err(SemanticError::ExpectedVectorWithSize(length))
    } else {
        Ok(list)
    }
}

impl TryFrom<Term> for Expr {
    type Error = SemanticError;

    fn try_from(value: Term) -> Result<Self, Self::Error> {
        DefMacro::try_new(value.clone())
            .or_else(|_| Recur::try_new(value.clone()))
            .or_else(|_| Def::try_new(value.clone()))
            .or_else(|_| Fun::try_new(value.clone()))
            .or_else(|_| Quote::try_new(value.clone()))
            .or_else(|_| Apply::try_new(value.clone()))
            .or_else(|_| List::try_new(value.clone()))
            .or_else(|_| Literal::try_new(value.clone()))
            .and_then(|value| value.ok_or(SemanticError::FailedToMatch))
    }
}

impl Display for Term {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.pretty_print(f, 0)
    }
}

impl From<String> for Term {
    fn from(s: String) -> Self {
        Term::String(s)
    }
}

impl From<usize> for Term {
    fn from(n: usize) -> Self {
        Term::Int(n as u64)
    }
}

impl From<Vec<Term>> for Term {
    fn from(terms: Vec<Term>) -> Self {
        Term::List(terms)
    }
}

#[macro_export]
macro_rules! soft_vec {
    ($($expr:expr),*) => {
        $crate::Expr::List($crate::List($crate::Term::Vec(vec![$($expr.into()),*])))
    };
}

#[macro_export]
macro_rules! keyword {
    ($str:literal) => {
        $crate::Expr::new_keyword($str)
    };
}

#[macro_export]
macro_rules! define_builtin {
    ($name:ident, $keyword:expr, $length:expr) => {
        impl $crate::ExprKind for $name {
            fn try_new(term: $crate::Term) -> $crate::Result<Option<$crate::Expr>> {
                let (head, tail) = term
                    .split()
                    .ok_or($crate::SemanticError::InvalidExpression)?;
                if head.is_keyword($keyword) {
                    let tail = $crate::assert_length(tail, $length)?;
                    Ok(Some($name(term.transport(tail.into())).into()))
                } else {
                    Ok(None)
                }
            }
        }
    };
    ($name:ident, $keyword:expr) => {
        impl $crate::ExprKind for $name {
            fn try_new(term: $crate::Term) -> $crate::Result<Option<$crate::Expr>> {
                let Some((head, tail)) = term.split() else {
                    return Ok(None);
                };
                if head.is_keyword($keyword) {
                    Ok(Some($name(term.transport(tail.into())).into()))
                } else {
                    Ok(Some($crate::Expr::from($name(term))))
                }
            }
        }
    };
}

#[macro_export]
macro_rules! define_ast {
    ($(#[$outer:meta])* $name:ident, {$($(#[$field_outer:meta])* $variant:ident),+}) => {
        $(#[$outer])*
        #[derive(Debug, Clone)]
        pub enum $name {
            $(
                $(#[$field_outer])*
                $variant($variant)
            ),+
        }

        impl From<$name> for $crate::Term {
            fn from(value: $name) -> Self {
                match value {
                    $(
                        $name::$variant(value) => value.into(),
                    )+
                }
            }
        }

        $(
            impl From<$variant> for $name {
                fn from(value: $variant) -> Self {
                    $name::$variant(value)
                }
            }

            $(#[$field_outer])*
            #[derive(Debug, Clone)]
            pub struct $variant(pub $crate::Term);

            impl From<$variant> for $crate::Term {
                fn from(value: $variant) -> Self {
                    value.0
                }
            }

            impl std::ops::Deref for $variant {
                type Target = $crate::Term;

                fn deref(&self) -> &Self::Target {
                    &self.0
                }
            }
        )+
    };
}
