use std::{iter::Peekable, str::Chars};

use crate::{SrcPos, Term};

pub fn is_identifier_char(c: char) -> bool {
    c != ' ' && c != '\n' && c != '\t' && c != '(' && c != ')' && c != '"' && c != ';'
}

pub struct Parser<'a> {
    pub peekable: Peekable<Chars<'a>>,
    pub string: &'a str,
    pub index: usize,
}

impl<'a> Parser<'a> {
    pub fn bump(&mut self) -> Option<char> {
        let c = self.peekable.next()?;
        self.index += c.len_utf8();
        Some(c)
    }

    pub fn peek(&mut self) -> Option<char> {
        self.peekable.peek().copied()
    }

    pub fn accumulate(&mut self, mut f: impl FnMut(char) -> bool) -> String {
        let mut string = String::new();

        loop {
            match self.peek() {
                Some(c) if f(c) => string.push(self.bump().unwrap()),
                _ => break,
            }
        }

        string
    }

    pub fn parse(&mut self) -> Result<Term, String> {
        let start = self.index;

        let result = match self.peek() {
            Some(c) if c.is_whitespace() => {
                self.accumulate(|c| c.is_whitespace());
                self.parse()
            }
            Some(';') => {
                self.accumulate(|c| c != '\n');
                self.parse()
            }
            Some('"') => {
                self.bump();
                let string = self.accumulate(|c| c != '"');

                if self.bump() != Some('"') {
                    return Err("expected '\"'".to_string());
                }

                Ok(Term::String(string))
            }
            Some(':') => {
                self.bump();
                let string = self.accumulate(is_identifier_char);
                Ok(Term::Atom(string))
            }
            Some(c) if c.is_ascii_digit() => {
                let string = self.accumulate(|c| c.is_ascii_digit());
                Ok(Term::Int(string.parse().unwrap()))
            }
            Some('(') => {
                self.bump();
                let mut terms = Vec::new();

                loop {
                    match self.peek() {
                        Some(c) if c.is_whitespace() => {
                            self.accumulate(|c| c.is_whitespace());
                        }
                        Some(')') => {
                            self.bump();
                            break;
                        }
                        Some(_) => {
                            terms.push(self.parse()?);
                        }
                        None => return Err("unexpected end of file".to_string()),
                    }
                }

                Ok(Term::List(terms))
            }
            Some(_) => {
                let string = self.accumulate(is_identifier_char);
                Ok(Term::Identifier(string))
            }
            None => Err("unexpected end of file".to_string()),
        }?;

        Ok(Term::SrcPos(
            SrcPos {
                byte: start..self.index,
                file: self.string.to_string(),
            },
            Box::new(result),
        ))
    }
}

pub fn parse_sexpr(string: &str) -> Result<Term, String> {
    let mut parser = Parser {
        peekable: string.chars().peekable(),
        string,
        index: 0,
    };

    parser.parse()
}

/// Tests for parser of S-expressions.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_string() {
        assert_eq!(
            parse_sexpr(r#""hello world""#).unwrap().unbox(),
            Term::String("hello world".to_string())
        );
    }
}
