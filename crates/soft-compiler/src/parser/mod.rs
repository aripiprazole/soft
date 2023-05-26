//! The main module of the parser structure. The main function here is the [parse] function that
//! parses a s-expression sequence into a vector of [Expr] structures.

use self::syntax::{Expr, ExprKind};
use self::tracker::Tracker;

use thiserror::Error;

pub mod syntax;
pub mod tracker;

/// An error generated by the parser.
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

    #[error("expected '('")]
    ExpectedList,
}

/// Type synonym for the result of the parser.
type Result<T = (), U = ParseError> = std::result::Result<T, U>;

/// The main parser structure. It holds a state with a source code.
pub struct Parser<'a> {
    tracker: Tracker<'a>,
    stack: Vec<Expr<'a>>,
    indices: Vec<usize>,
}

/// Checks if a character is a valid identifier character. A valid identifier character is any
/// character that is not a whitespace, a parenthesis or a colon (so it's not a symbol) and not a
/// double quote (so it's not a string).
#[inline(always)]
fn is_identifier_char(c: char) -> bool {
    !c.is_whitespace() && !matches!(c, '(' | ')' | ':' | '"')
}

impl<'a> Parser<'a> {
    pub fn new(code: &'a str) -> Self {
        Self {
            tracker: Tracker::new(code),
            stack: vec![],
            indices: vec![],
        }
    }

    #[inline(always)]
    fn save_jump(&mut self) {
        self.tracker.save_jump();
    }

    #[inline(always)]
    fn peek(&mut self) -> Option<char> {
        self.tracker.peek()
    }

    #[inline(always)]
    fn next(&mut self) -> Option<char> {
        self.tracker.next_char()
    }

    #[inline(always)]
    fn save(&mut self) {
        self.tracker.save();
    }

    #[inline(always)]
    fn jump(&mut self) {
        self.tracker.jump();
    }

    fn parse_number(&mut self) -> Result {
        self.save_jump();

        let mut num = 0;

        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                num *= 10;
                num += c as u64 - '0' as u64;
                self.next();
            } else {
                break;
            }
        }

        let expr = Expr::new(self.tracker.pop_range(), ExprKind::Number(num));
        self.stack.push(expr);

        Ok(())
    }

    fn parse_lpar(&mut self) -> Result {
        self.save_jump();
        self.indices.push(self.stack.len());
        Ok(())
    }

    fn parse_rpar(&mut self) -> Result {
        self.save_jump();
        let index = self.indices.pop().unwrap();
        let stack = self.stack.split_off(index);
        let expr = Expr::new(self.tracker.pop_range(), ExprKind::List(stack));
        self.stack.push(expr);
        Ok(())
    }

    fn parse_identifier(&mut self) -> Result {
        self.save_jump();

        while self.peek().map(is_identifier_char).unwrap_or_default() {
            self.tracker.next_char();
        }

        let range = self.tracker.pop_range();
        let expr = Expr::new(
            range.clone(),
            ExprKind::Identifier(self.tracker.substring(range)),
        );
        self.stack.push(expr);

        Ok(())
    }

    fn parse_symbol(&mut self) -> Result {
        self.next();
        self.save();

        while self
            .tracker
            .peek()
            .map(is_identifier_char)
            .unwrap_or_default()
        {
            self.next();
        }

        let range = self.tracker.pop_range();

        let expr = Expr::new(range.clone(), ExprKind::Atom(self.tracker.substring(range)));

        self.stack.push(expr);

        Ok(())
    }

    fn parse_string(&mut self) -> Result {
        self.next();
        self.save();

        while self.peek().map(|c| c != '"').unwrap_or_default() {
            self.next();
        }

        if self.peek().is_none() {
            Err(ParseError::UnexpectedEOF)
        } else {
            Ok(())
        }
    }

    #[inline(always)]
    fn parse_item(&mut self, peek: char) -> Result {
        match peek {
            c if c.is_whitespace() => self.jump(),
            '0'..='9' => self.parse_number()?,
            '(' => self.parse_lpar()?,
            ')' => self.parse_rpar()?,
            ':' => self.parse_symbol()?,
            '"' => self.parse_string()?,
            _ => self.parse_identifier()?,
        };
        Ok(())
    }

    fn parse(&mut self) -> Result {
        while let Some(peek) = self.peek() {
            self.parse_item(peek)?;
        }
        Ok(())
    }
}

/// Main function of the parser. It parses a s-expression sequence into a vector of [Expr] otherwise
/// it returns a parsing error.
pub fn parse(code: &str) -> Result<Vec<Expr>> {
    let mut parser = Parser::new(code);

    parser.parse()?;

    Ok(std::mem::take(&mut parser.stack))
}
