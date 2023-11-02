use crate::Term;

define_ast!(Expression, {
    Fun,         // (fun (a b) (+ a b))
    List,        // (a b c) or (list a b c)
    Appl,        // (a b c) or (apply a b c)
    Define,      // (define a 123)
    MacroDefine, // (macro-define a (fun (a b) (+ a b))
    Quote,       // '(fun (a b) (+ a b))
    Literal      // 123 | "bla" | :bla | bla
});

define_builtin!(MacroDefine, "macro-define", 2);
define_builtin!(Define, "define", 2);
define_builtin!(Fun, "fun", 2);
define_builtin!(Quote, "'", 2);
define_builtin!(List, "list");
define_builtin!(Appl, "apply");

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
        pub fn body(&self) -> Result<Expression> {
            self.0.at(2).ok_or(SemanticError::MissingBody)?.try_into()
        }
    }
}

/// List expression construct, it's a list of expressions.
pub mod list {
    use super::*;

    impl List {
        /// Returns a list of expressions that are in the spine of the list.
        pub fn spine(&self) -> Result<Vec<Expression>> {
            self.0
                .spine()
                .ok_or(SemanticError::InvalidList)?
                .into_iter()
                .map(Expression::try_from)
                .collect()
        }
    }
}

/// Appl expression construct, it's a function application.
pub mod appl {
    use super::*;

    impl Appl {
        /// Returns the callee of the application.
        pub fn callee(&self) -> Result<Expression> {
            self.0.at(0).ok_or(SemanticError::MissingHead)?.try_into()
        }

        /// Returns a list of arguments that are in the spine of the application.
        pub fn spine(&self) -> Result<Vec<Expression>> {
            self.0
                .spine()
                .ok_or(SemanticError::InvalidArguments)?
                .into_iter()
                .skip(1) // Skip the head of the application.
                .map(Expression::try_from)
                .collect()
        }
    }
}

/// Define expression construct, it's a definition of a value.
pub mod define {
    pub use super::*;

    impl Define {
        /// Returns the name of the definition.
        pub fn name(&self) -> Result<Expression> {
            self.0
                .at(1)
                .ok_or(SemanticError::InvalidExpression)?
                .try_into()
        }

        /// Returns the value of the definition.
        pub fn value(&self) -> Result<Expression> {
            self.0
                .at(2)
                .ok_or(SemanticError::InvalidExpression)?
                .try_into()
        }
    }
}

/// Macro define expression construct, it's a definition of a value.
pub mod macro_define {
    pub use super::*;

    impl MacroDefine {
        /// Returns the name of the definition.
        pub fn name(&self) -> Result<Expression> {
            self.0
                .at(1)
                .ok_or(SemanticError::InvalidExpression)?
                .try_into()
        }

        /// Returns the value of the definition.
        pub fn value(&self) -> Result<Expression> {
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
        pub fn expression(&self) -> Result<Expression> {
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
        fn try_new(term: Term) -> Result<Option<Expression>> {
            Ok(Some(Expression::Literal(Literal(term))))
        }
    }

    impl Expression {
        /// Expects a string literal and returns it's value.
        pub fn string(&self) -> Result<String> {
            match self {
                Expression::Literal(Literal(Term::String(string))) => Ok(string.clone()),
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

impl TryFrom<Term> for Expression {
    type Error = SemanticError;

    fn try_from(value: Term) -> Result<Self, Self::Error> {
        try_new!(value, [
            Define,
            MacroDefine,
            Fun,
            Appl,
            List,
            Quote,
            Literal
        ])
        .ok_or(SemanticError::InvalidExpression)
    }
}

pub trait ExpressionKind: Sized {
    fn try_new(term: Term) -> Result<Option<Expression>>;
}

/// Pipes try new functions to each other, it's useful to create a try_from function for enums
/// that have multiple variants.
macro_rules! try_new {
    ($target:expr, [$($value:ident),+]) => {{
        use crate::semantic::ExpressionKind;
        let mut value = None;
        $(value = match value {
            Some(value) => Some(value),
            None => $value::try_new($target.clone())?,
        };)+
        value
    }};
}

macro_rules! define_builtin {
    ($name:ident, $keyword:expr, $length:expr) => {
        impl $crate::semantic::ExpressionKind for $name {
            fn try_new(
                term: $crate::Term,
            ) -> $crate::semantic::Result<Option<$crate::semantic::Expression>> {
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
            ) -> $crate::semantic::Result<Option<$crate::semantic::Expression>> {
                let Some((head, tail)) = term.split() else {
                    return Ok(None);
                };
                if head.is_keyword($keyword) {
                    Ok(Some($name(term.transport(tail.into())).into()))
                } else {
                    Ok(Some($crate::semantic::Expression::from($name(term))))
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
use try_new;
