//! The reader is responsible for parsing the input string into a list of expressions. The reader
//! main function is [read].

use std::{fmt::Display, iter::Peekable, str::Chars};

use crate::error::{Result, RuntimeError};
use crate::value::{Expr, ExprKind, Location, Value};

/// A prefix is a symbol that can be at the beggining of an expression. It is used to create quote
/// and unquote expressions.
enum Prefix {
    Quote,
    Unquote,
}

impl Display for Prefix {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Prefix::Quote => write!(f, "quote"),
            Prefix::Unquote => write!(f, "unquote"),
        }
    }
}

/// A state is a mutable object that is used to keep track of the current state of the reader.
pub struct State<'a> {
    peekable: Peekable<Chars<'a>>,
    stack: Vec<Value>,
    indices: Vec<usize>,
    prefix: Vec<(Prefix, Location, usize)>,
    position: Location,
}

impl<'a> State<'a> {
    fn new(input: &'a str, file: Option<String>) -> Self {
        Self {
            peekable: input.chars().peekable(),
            stack: Vec::new(),
            prefix: Vec::new(),
            indices: Vec::new(),
            position: Location {
                line: 1,
                column: 0,
                file,
            },
        }
    }

    fn advance(&mut self) -> Option<char> {
        let char = self.peekable.next()?;

        match char {
            '\n' => {
                self.position.line += 1;
                self.position.column = 0;
            }
            _ => self.position.column += 1,
        }

        Some(char)
    }

    fn push(&mut self, value: Value) {
        self.stack.push(value);
    }

    fn open(&mut self) {
        self.indices.push(self.stack.len());
    }

    fn close(&mut self) {
        let index = self.indices.pop().unwrap();
        let values = self.stack.split_off(index);

        let expr = Value::from_iter(values.into_iter(), self.position.clone().into());

        self.stack.push(expr);
    }

    fn prefix(&mut self, start: Location, prefix: Prefix) {
        self.prefix.push((prefix, start, self.indices.len()));
    }

    fn prefix_close(&mut self) -> Result<()> {
        if let Some((_, _, place)) = self.prefix.last() {
            if self.indices.len() == *place {
                let (prefix, loc, _) = self.prefix.pop().unwrap();
                if let Some(expr) = self.stack.pop() {
                    let span = expr.span.clone();
                    let expr = Value::from_iter(
                        vec![
                            Expr::new(ExprKind::Id(prefix.to_string()), loc.into()).into(),
                            expr,
                        ]
                        .into_iter(),
                        span,
                    );
                    self.stack.push(expr);
                } else {
                    return Err(RuntimeError::UnmatchedQuote(self.position.clone()));
                }
            }
        }
        Ok(())
    }

    fn accumulate_while<F>(&mut self, chr: char, mut f: F) -> String
    where
        F: FnMut(char) -> bool,
    {
        let mut string: String = chr.into();

        while let Some(&char) = self.peekable.peek() {
            if f(char) {
                string.push(self.advance().unwrap());
            } else {
                break;
            }
        }

        string
    }

    fn read(&mut self) -> Result<Vec<Value>> {
        while let Some(chr) = self.advance() {
            let start = self.position.clone();
            match chr {
                ' ' | '\n' | '\r' | '\t' => continue,
                '\'' => {
                    self.prefix(start, Prefix::Quote);
                    continue;
                }
                ',' => {
                    self.prefix(start, Prefix::Unquote);
                    continue;
                }
                ';' => {
                    self.parse_comment();
                    continue;
                }
                '(' => self.open(),
                ')' => self.close(),
                '"' => self.parse_string(&start)?,
                _ => self.parse_rest(start, chr),
            }
            self.prefix_close()?;
        }

        if !self.prefix.is_empty() {
            return Err(RuntimeError::UnmatchedQuote(self.position.clone()));
        }

        if !self.indices.is_empty() {
            return Err(RuntimeError::UnclosedParenthesis(self.position.clone()));
        }

        Ok(std::mem::take(&mut self.stack))
    }

    fn parse_rest(&mut self, start: Location, chr: char) {
        let string = self.accumulate_while(chr, |c| {
            !matches!(c, '\n' | '\r' | '\t' | ' ' | ')' | '(' | '"')
        });

        if let Ok(int) = string.parse::<i64>() {
            self.push(Expr::new(ExprKind::Int(int), start.into()).into());
        } else {
            self.push(Expr::new(ExprKind::Id(string), start.into()).into());
        }
    }

    fn parse_comment(&mut self) {
        self.accumulate_while(';', |c| c != '\n');
        self.advance();
    }

    fn parse_string(&mut self, start: &Location) -> Result<()> {
        let string = self.accumulate_while('"', |c| c != '"');
        if self.advance().is_none() {
            return Err(RuntimeError::UnclosedString(self.position.clone()));
        }
        self.push(Expr::new(ExprKind::Str(string), start.clone().into()).into());
        self.advance();
        Ok(())
    }
}

/// Read a string and return a list of expressions.
pub fn read(input: &str, file: Option<String>) -> Result<Vec<Value>> {
    let mut state = State::new(input, file);
    state.read()
}
