//! Parser for the soft language. This parser uses a stack to keep track of each of the expressions
//! instead of using recursion. This is because the soft language is easy to parse (as it's a lisp).

use crate::syntax::{Expr, Point, Range};
use std::{iter::Peekable, mem, str::Chars};

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

/// Checks if it's a reserved letter of the language. It's used for parsing identifiers.
pub fn is_reserved_char(c: char) -> bool {
    matches!(c, '(' | ')' | ':')
}

/// Checks if it's a valid identifier character. A valid identifier is anything that is not a 
/// whitespace or a reserved character.
pub fn is_identifier_char(c: char) -> bool {
    !c.is_whitespace() && !is_reserved_char(c)
}

/// The parser for the soft language. It uses a stack to keep track
pub struct Parser<'a> {
    iterator: Peekable<Chars<'a>>,
    start: Point,
    current: Point,
    stack: Vec<Expr<'a>>,
    indices: Vec<(usize, Point)>,
    code: &'a str,
    index: usize
}

impl<'a> Parser<'a> {
    pub fn new(code: &'a str) -> Self {
        Self {
            iterator: code.chars().peekable(),
            start: Default::default(),
            current: Default::default(),
            stack: Default::default(),
            indices: Default::default(),
            code,
            index: 0,
        }
    }

    pub fn next_char(&mut self) -> Option<char> {
        let chr = self.iterator.next();
        if let Some(char) = chr {
            self.current.advance(char)
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
            self.next_char();
        }

        Ok(Expr::Num(self.range(), num))
    }

    pub fn parse_identifier(&mut self, mut id: usize) -> Result<usize, ParseError> {
        while let Some(&c) = self.peek() {
            if is_identifier_char(c) {
                id += c.len_utf8();
                self.index += c.len_utf8();
                self.next_char();
            } else {
                break;
            }
        }
        Ok(id)
    }

    pub fn parse_id(&mut self) -> Result<Expr, ParseError> {
        let str = self.parse_identifier(1)?;
        Ok(Expr::Id(self.range(), &self.code[self.index..self.index+str]))
    }

    pub fn parse_atom(&mut self) -> Result<Expr, ParseError> {
        let str = self.parse_identifier(0)?;
        Ok(Expr::Symbol(self.range(), &self.code[self.index..self.index+str]))
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

    pub fn parse_lparen(&mut self) {
        self.indices.push((self.stack.len(), self.start));
        self.set_start()
    }

    pub fn parse_rparen(&mut self) -> Result<Expr, ParseError> {
        self.indices
            .pop()
            .ok_or(ParseError::UnmatchedParenthesis)
            .map(|(place, start)| {
                Expr::List(Range::new(start, self.current), self.stack.split_off(place))
            })
    }

    /// The entry point of the parser, it parsers the whole code as a composite of s-exprs.
    pub fn parse(&mut self) -> Result<Vec<Expr>, ParseError> {
        while let Some(c) = self.next_char() {
            let result = match c {
                x if x.is_whitespace() => continue,
                '(' => {
                    self.parse_lparen();
                    continue;
                }
                ')' => self.parse_rparen(),
                '0'..='9' => self.parse_number(c),
                '"' => self.parse_string(),
                ':' => self.parse_atom(),
                _ => self.parse_id(),
            };
            self.stack.push(result?);
            self.set_start();
        }

        if self.indices.is_empty() {
            Ok(mem::take(&mut self.stack))
        } else {
            Err(ParseError::UnmatchedClosingParenthesis)
        }
    }
}

/// The entrypoint of parsing, it just gets a source code written as S expressions and turn it into
/// a sequence of [Expr]. It is a wrapper over [Parser::parse].
pub fn parse(code: &str) -> Result<Vec<Expr>, ParseError> {
    Parser::new(code).parse()
}
