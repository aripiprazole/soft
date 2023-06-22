use thiserror::Error;

use crate::value::{Location, Value};

#[derive(Error, Debug)]
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

    #[error("expected an identifier but got '{0}'")]
    ExpectedIdentifier(String),

    #[error("expected a list but got '{0}'")]
    ExpectedList(String),

    #[error("expected a number but got '{0}'")]
    ExpectedNumber(String),

    #[error("invalid escape")]
    InvalidEscape,

    #[error("unterminated string")]
    UnterminatedString,
}

pub type Result<T> = std::result::Result<T, RuntimeError>;
