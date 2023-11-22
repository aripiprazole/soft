use crate::Term;

define_ast!(Expr, {
    Fun,      // (fun* (a b) (+ a b))
    List,     // [a b c] or (list a b c)
    Apply,    // (a b c) or (apply a b c)
    Def,      // (def a 123)
    Recur,    // (recur a)
    Deref,    // (deref* 123)
    Atomic,   // (atomic* 123)
    Set,      // (set* a 123)
    DefMacro, // (defmacro* a (fun (a b) (+ a b))
    Quote,    // '(fun (a b) (+ a b))
    Literal   // 123 | "bla" | :bla | bla
});

define_builtin!(DefMacro, "defmacro*", 2);
define_builtin!(Def, "def*", 2);
define_builtin!(Recur, "recur");
define_builtin!(Atomic, "atomic*", 1);
define_builtin!(Deref, "deref*", 1);
define_builtin!(Set, "set*", 2);
define_builtin!(Fun, "fun*", 2);
define_builtin!(Quote, "'", 2);
define_builtin!(Apply, "apply");

pub type Result<T, E = SemanticError> = std::result::Result<T, E>;

/// Fun expression constructs, it's a function that has a list of parameters and a body.
pub mod fun {
    use super::*;

    impl Fun {
        /// Returns a list of parameters that are in the spine of the function.
        pub fn parameters(&self) -> Result<List> {
            self.0
                .at(1)
                .map(List)
                .ok_or(SemanticError::MissingParameters)
        }

        /// Returns the body of the function.
        pub fn body(&self) -> Result<Expr> {
            self.0.at(2).ok_or(SemanticError::MissingBody)?.try_into()
        }
    }
}

/// List expression construct, it's a list of expressions.
pub mod list {
    use SemanticError::*;

    use super::*;

    impl ExpressionKind for List {
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
        pub fn spine(&self) -> Result<Vec<Expr>> {
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

/// Set expression construct, it's a definition of a value.
pub mod set {
    pub use super::*;

    impl Set {
        /// Returns the name of the definition.
        pub fn target(&self) -> Result<Expr> {
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

/// Deref expression construct, it's a mutable value.
pub mod deref {
    pub use super::*;

    impl Deref {
        /// Returns the value of the definition.
        pub fn value(&self) -> Result<Expr> {
            self.0
                .at(1)
                .ok_or(SemanticError::InvalidExpression)?
                .try_into()
        }
    }
}

/// Atomic expression construct, it's a mutable value.
pub mod atomic {
    pub use super::*;

    impl Atomic {
        /// Returns the value of the definition.
        pub fn value(&self) -> Result<Expr> {
            self.0
                .at(1)
                .ok_or(SemanticError::InvalidExpression)?
                .try_into()
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
        pub fn expression(&self) -> Result<Expr> {
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

    impl ExpressionKind for Literal {
        fn try_new(term: Term) -> Result<Option<Expr>> {
            Ok(Some(Expr::Literal(Literal(term))))
        }
    }

    impl Expr {
        /// Expects a string literal and returns it's value.
        pub fn string(&self) -> Result<String> {
            match self {
                Expr::Literal(Literal(Term::String(string))) => Ok(string.clone()),
                _ => Err(SemanticError::ExpectedString),
            }
        }
    }
}

#[derive(thiserror::Error, Debug, Clone)]
pub enum SemanticError {
    #[error("invalid expression")]
    InvalidExpression,

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
            .or_else(|_| Atomic::try_new(value.clone()))
            .or_else(|_| Deref::try_new(value.clone()))
            .or_else(|_| Recur::try_new(value.clone()))
            .or_else(|_| Def::try_new(value.clone()))
            .or_else(|_| Set::try_new(value.clone()))
            .or_else(|_| Fun::try_new(value.clone()))
            .or_else(|_| Quote::try_new(value.clone()))
            .or_else(|_| Apply::try_new(value.clone()))
            .or_else(|_| List::try_new(value.clone()))
            .or_else(|_| Literal::try_new(value.clone()))?
            .ok_or(SemanticError::InvalidExpression)
    }
}

pub trait ExpressionKind: Sized {
    fn try_new(term: Term) -> Result<Option<Expr>>;
}

macro_rules! define_builtin {
    ($name:ident, $keyword:expr, $length:expr) => {
        impl $crate::semantic::ExpressionKind for $name {
            fn try_new(
                term: $crate::Term,
            ) -> $crate::semantic::Result<Option<$crate::semantic::Expr>> {
                let (head, tail) = term
                    .split()
                    .ok_or($crate::semantic::SemanticError::InvalidExpression)?;
                if head.is_keyword($keyword) {
                    let tail = $crate::semantic::assert_length(tail, $length)?;
                    Ok(Some($name(term.transport(tail.into())).into()))
                } else {
                    Ok(None)
                }
            }
        }
    };
    ($name:ident, $keyword:expr) => {
        impl $crate::semantic::ExpressionKind for $name {
            fn try_new(
                term: $crate::Term,
            ) -> $crate::semantic::Result<Option<$crate::semantic::Expr>> {
                let Some((head, tail)) = term.split() else {
                    return Ok(None);
                };
                if head.is_keyword($keyword) {
                    Ok(Some($name(term.transport(tail.into())).into()))
                } else {
                    Ok(Some($crate::semantic::Expr::from($name(term))))
                }
            }
        }
    };
}

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

        $(
            impl From<$variant> for $name {
                fn from(value: $variant) -> Self {
                    $name::$variant(value)
                }
            }

            $(#[$field_outer])*
            #[derive(Debug, Clone)]
            pub struct $variant(crate::Term);

            impl std::ops::Deref for $variant {
                type Target = crate::Term;

                fn deref(&self) -> &Self::Target {
                    &self.0
                }
            }
        )+
    };
}

use define_ast;
use define_builtin;