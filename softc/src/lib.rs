use std::{iter::Peekable, str::Chars};

/// Term is a recursive data structure that represents a list of terms, an atom, an identifier,
/// or an integer.
///
/// It's the first part of our Abstract-Syntax-Tree (AST).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Term {
    List(Vec<Term>),    // (a b c)
    Atom(String),       // :bla
    Identifier(String), // bla
    Int(u64),           // 123
    Float(u64, u64),    // 123.456
    String(String),     // "some stuff"
    SrcPos(SrcPos, Box<Term>),
}

impl From<Vec<Term>> for Term {
    fn from(terms: Vec<Term>) -> Self {
        Term::List(terms)
    }
}

impl Term {
    pub fn transport(self, with: Term) -> Term {
        if let Term::SrcPos(src_pos, _) = self {
            Term::SrcPos(src_pos, with.into())
        } else {
            with
        }
    }

    pub fn at(&self, nth: usize) -> Option<Term> {
        if let Term::List(ls) = self {
            ls.get(nth).cloned()
        } else {
            None
        }
    }

    pub fn split(&self) -> Option<(Term, Vec<Term>)> {
        if let Term::List(ls) = self {
            let (first, rest) = ls.split_first()?;
            Some((first.clone(), rest.to_vec()))
        } else {
            None
        }
    }

    pub fn is_keyword(&self, keyword: &str) -> bool {
        matches!(self, Term::Identifier(x) if x == keyword)
    }

    pub fn spine(&self) -> Option<Vec<Term>> {
        if let Term::List(ls) = self.clone() {
            Some(ls.clone())
        } else {
            None
        }
    }

    /// Removes meta information from a term.
    pub fn unbox(self) -> Term {
        match self {
            Term::SrcPos(_, t) => t.unbox(),
            Term::List(x) => Term::List(x.into_iter().map(|t| t.unbox()).collect()),
            t => t,
        }
    }
}

/// Meta information about a term, or any other part of the AST.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SrcPos {
    pub byte: std::ops::Range<usize>,
    pub file: String,
}

impl SrcPos {
    pub fn next(&mut self, c: char) {
        self.byte.end += c.len_utf8();
    }

    pub fn reset(&mut self) {
        self.byte.start = self.byte.end;
    }
}

/// Semantic expression abstract syntax.
pub mod semantic;
pub mod runtime;
pub mod allocator;
pub mod codegen;
pub mod pprint;

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

        println!("{}", parse_sexpr(r#"((((((((((((("hello world" 2 3 ) 4 5 ) 9 1)) 23 3 42)))))))))"#).unwrap().unbox());
    }

    #[test]
    fn parses_sexpr() {
        println!("{:?}", parse_sexpr(r#"(a b c)"#).unwrap());
    }
}
