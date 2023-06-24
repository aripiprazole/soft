/// Definitions of errors that can occur during runtime.
use thiserror::Error;

use crate::value::{Expr, Location, Value};

#[derive(Error, Debug, Clone)]
pub enum RuntimeError {
    #[error("undefined name '{0}'")]
    UndefinedName(String),

    #[error("cannot call as function '{0}'")]
    NotCallable(Value),

    #[error("unmatched parenthesis")]
    UnmatchedParenthesis(Location),

    #[error("unclosed parenthesis")]
    UnclosedParenthesis(Location),

    #[error("unclosed string")]
    UnclosedString(Location),

    #[error("unmatched quote")]
    UnmatchedQuote(Location),

    #[error("wrong arity, expected {0} arguments, got {1}")]
    WrongArity(usize, usize),

    #[error("expected an identifier but got {0}")]
    ExpectedIdentifier(String),

    #[error("expected an err but got {0}")]
    ExpectedErr(String),

    #[error("expected a string but got {0}")]
    ExpectedString(String),

    #[error("expected a list but got {0}")]
    ExpectedList(String),

    #[error("expected a number but got {0}")]
    ExpectedNumber(String),

    #[error("{0}")]
    UserError(Value),

    #[error("invalid escape")]
    InvalidEscape,

    #[error("unterminated string")]
    UnterminatedString,

    #[error("catch requires two arguments")]
    CatchRequiresTwoArgs,
}

impl From<String> for RuntimeError {
    fn from(value: String) -> Self {
        RuntimeError::UserError(Value::from(Expr::Str(value)))
    }
}

impl From<&str> for RuntimeError {
    fn from(value: &str) -> Self {
        RuntimeError::UserError(Value::from(Expr::Str(value.to_string())))
    }
}

pub type Result<T> = std::result::Result<T, RuntimeError>;
