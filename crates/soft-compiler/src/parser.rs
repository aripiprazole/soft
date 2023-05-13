//! Parser for the soft language. This parser
//! uses a stack to keep track of each of the expressions
//! instead of using recursion. This is because the
//! soft language is easy to parse (as it's a lisp).

use crate::syntax::{Expr, Point, Range};
use std::{iter::Peekable, str::Chars};

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("Unknown escape sequence: \\{0}")]
    UnknownEscape(char),

    #[error("Unterminated string")]
    UnterminatedString,

    #[error("Unmatched '('")]
    UnmatchedParenthesis,

    #[error("Unmatched ')'")]
    UnmatchedClosingParenthesis,

    #[error("Invalid character: {0}")]
    InvalidCharacter(char),
}

pub fn is_reserved(c: char) -> bool {
    matches!(c, '(' | ')' | ':')
}

pub fn is_identifier(c: char) -> bool {
    !c.is_whitespace() && !is_reserved(c)
}

pub struct Parser<'a> {
    iterator: Peekable<Chars<'a>>,
    start: Point,
    current: Point
}

impl<'a> Parser<'a> {
    pub fn new(code: &'a str) -> Self {
        Self {
            iterator: code.chars().peekable(),
            start: Default::default(),
            current: Default::default(),
        }
    }

    pub fn next(&mut self) -> Option<char> {
        let chr = self.iterator.next();
        match &chr {
            Some('\n') => {
                self.current.line += 1;
                self.current.column = 0;
            },
            Some(_) => {
                self.current.column += 1;
            },
            _ => ()
        }
        chr
    }

    pub fn peek(&mut self) -> Option<&char> {
        self.iterator.peek()
    }

    pub fn set_start(&mut self) {
        self.start = self.current;
    }

    pub fn range(&self) -> Range {
        Range::new(self.start, self.current)
    }

    pub fn parse_number(&mut self, c: char) -> Result<Expr, ParseError> {
        let mut num = c.to_digit(10).unwrap() as u64;
        while let Some(digit) = self.peek().and_then(|c| c.to_digit(10)) {
            num = num * 10 + digit as u64;
            self.next();
        }

        Ok(Expr::Num(self.range(), num))
    }

    pub fn parse_identifier(&mut self, mut id: String) -> Result<String, ParseError> {
        while let Some(&c) = self.peek() {
            if is_identifier(c) {
                id.push(c);
                self.next();
            } else {
                break;
            }
        }
        Ok(id)
    }

    pub fn parse_id(&mut self, c: char) -> Result<Expr, ParseError> {
        let str = self.parse_identifier(c.to_string())?;
        Ok(Expr::Id(self.range(), str))
    }

    pub fn parse_atom(&mut self) -> Result<Expr, ParseError> {
        let str = self.parse_identifier(Default::default())?;
        Ok(Expr::Symbol(self.range(), str))
    }

    pub fn parse_char(&mut self) -> Result<char, ParseError> {
        match self.iterator.next() {
            Some('n') => Ok('\n'),
            Some('r') => Ok('\r'),
            Some('t') => Ok('\t'),
            Some('"') => Ok('"'),
            Some('\\') => Ok('\\'),
            Some(c) => Err(ParseError::UnknownEscape(c)),
            None => Err(ParseError::UnterminatedString),
        }
    }

    pub fn parse_string(&mut self) -> Result<Expr, ParseError> {
        let mut string = String::new();
        while let Some(c) = self.iterator.next() {
            match c {
                '"' => break,
                '\\' => string.push(self.parse_char()?),
                _ => string.push(c),
            }
        }

        if self.peek().is_none() {
            Err(ParseError::UnterminatedString)
        } else {
            Ok(Expr::Str(self.range(), string))
        }
    }

    pub fn parse(&mut self) -> Result<Vec<Expr>, ParseError> {
        let mut stack = Vec::new();
        let mut indices = Vec::new();

        while let Some(c) = self.next() {
            let result = match c {
                x if x.is_whitespace() => continue,
                '(' => {
                    indices.push((stack.len(), self.start));
                    self.set_start();
                    continue;
                }
                ')' => indices
                    .pop()
                    .ok_or(ParseError::UnmatchedParenthesis)
                    .map(|(place, start)| Expr::List(Range::new(start, self.current), stack.split_off(place))),
                '0'..='9' => self.parse_number(c),
                '"' => self.parse_string(),
                ':' => self.parse_atom(),
                _ => self.parse_id(c)
            };
            stack.push(result?);
            self.set_start();
        }

        if indices.is_empty() {
            Ok(stack)
        } else {
            Err(ParseError::UnmatchedClosingParenthesis)
        }
    }
}

pub fn parse(code: &str) -> Result<Vec<Expr>, ParseError> {
    Parser::new(code).parse()
}