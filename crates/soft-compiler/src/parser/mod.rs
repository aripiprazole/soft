//! The main module of the parser structure. The main function here is the [parse] function that
//! parses a s-expression sequence into a [Vec<Expr>]

use crate::syntax::{Expr, ExprKind};

use self::tracker::Tracker;

use thiserror::Error;
mod tracker;

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("unclosed parenthesis")]
    UnclosedParenthesis,

    #[error("unexpected parenthesis")]
    ExtraParenthesis,

    #[error("unexpected end of file")]
    UnexpectedEOF,

    #[error("unfinished string")]
    UnfinishedString,
}

pub type Result<U, T = ()> = std::result::Result<T, U>;

pub struct Parser<'a> {
    tracker: Tracker<'a>,
    stack: Vec<Expr<'a>>,
    indices: Vec<usize>,
}

pub fn is_identifier_char(c: &char) -> bool {
    !c.is_whitespace() && !matches!(c, '(' | ')' | ':')
}

impl<'a> Parser<'a> {
    pub fn new(code: &'a str) -> Self {
        Self {
            tracker: Tracker::new(code),
            stack: vec![],
            indices: vec![],
        }
    }

    pub fn parse_number(&mut self) -> Result<ParseError> {
        self.tracker.save_jump();

        let mut num = 0;

        while let Some(c) = self.tracker.peek() {
            if c.is_ascii_digit() {
                num *= 10;
                num += *c as u64 - '0' as u64;
                self.tracker.next();
            } else {
                break;
            }
        }

        let expr = Expr::new(ExprKind::Num(num), self.tracker.pop_range());
        self.stack.push(expr);

        Ok(())
    }

    pub fn parse_lpar(&mut self) -> Result<ParseError> {
        self.tracker.save_jump();
        self.indices.push(self.stack.len());

        Ok(())
    }

    pub fn parse_rpar(&mut self) -> Result<ParseError> {
        self.tracker.save_jump();
        let index = self.indices.pop().unwrap();
        let stack = self.stack.split_off(index);
        let expr = Expr::new(ExprKind::List(stack), self.tracker.pop_range());
        self.stack.push(expr);

        Ok(())
    }

    pub fn parse_identifier(&mut self) -> Result<ParseError> {
        self.tracker.save_jump();

        while self
            .tracker
            .peek()
            .map(is_identifier_char)
            .unwrap_or_default()
        {
            self.tracker.next();
        }

        let range = self.tracker.pop_range();
        let expr = Expr::new(ExprKind::Id(self.tracker.substring(range.clone())), range);
        self.stack.push(expr);

        Ok(())
    }

    pub fn parse_symbol(&mut self) -> Result<ParseError> {
        self.tracker.next();
        self.tracker.save();

        while self
            .tracker
            .peek()
            .map(is_identifier_char)
            .unwrap_or_default()
        {
            self.tracker.next();
        }

        let range = self.tracker.pop_range();
        let expr = Expr::new(ExprKind::Symbol(self.tracker.substring(range.clone())), range);
        self.stack.push(expr);

        Ok(())
    }

    pub fn parse_string(&mut self) -> Result<ParseError> {
        self.tracker.next();
        self.tracker.save();

        while self.tracker.peek().map(|c| *c != '"').unwrap_or_default() {
            self.tracker.next();
        }

        if self.tracker.peek().is_none() {
            Err(ParseError::UnexpectedEOF)
        } else {
            Ok(())
        }
    }

    pub fn parse(&mut self) -> Result<ParseError> {
        while let Some(char) = self.tracker.peek() {
            match char {
                c if c.is_whitespace() => self.tracker.jump(),
                '0'..='9' => self.parse_number()?,
                '(' => self.parse_lpar()?,
                ')' => self.parse_rpar()?,
                ':' => self.parse_symbol()?,
                '"' => self.parse_string()?,
                _ => self.parse_identifier()?,
            };
        }
        Ok(())
    }
}

pub fn parse(code: &str) -> Result<ParseError, Vec<Expr>> {
    let mut parser = Parser::new(code);

    parser.parse()?;

    Ok(std::mem::take(&mut parser.stack))
}
