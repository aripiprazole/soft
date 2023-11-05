use std::fmt::Display;

/// Semantic expression abstract syntax.
pub mod semantic;
pub mod runtime;
pub mod allocator;
pub mod codegen;
pub mod parser;

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

impl Display for Term {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.pretty_print(f, 0)
    }
}

impl From<Vec<Term>> for Term {
    fn from(terms: Vec<Term>) -> Self {
        Term::List(terms)
    }
}

impl Term {
    fn width(&self) -> usize {
        match self {
            Term::List(s) => {
                let mut width = 2;
                for t in s {
                    width += t.width();
                }
                width
            },
            Term::Atom(s) => s.len(),
            Term::Identifier(s) => s.len(),
            Term::Int(n) => n.to_string().len(),
            Term::Float(n, u) => n.to_string().len() + u.to_string().len() + 1,
            Term::String(s) => s.len(),
            Term::SrcPos(_, t) => t.width(),
        }
    }

    fn pretty_print(&self, f: &mut std::fmt::Formatter<'_>, indent: usize)-> std::fmt::Result  {
        match self {
            Term::List(s) => {
                if self.width() + indent > 80 {
                    write!(f, "{:indent$}(", "", indent = indent)?;
                    for t in s {
                        write!(f, "{:indent$}", "", indent = indent + 2)?;
                        t.pretty_print(f, indent + 2)?;
                        writeln!(f)?;
                    }
                    write!(f, "{:indent$})", "", indent = indent)?;
                }

                Ok(())
            },
            Term::Atom(s) => write!(f, ":{}", s),
            Term::Identifier(s) => write!(f, "{}", s),
            Term::Int(s) => write!(f, "{}", s),
            Term::Float(u, n) => write!(f, "{}.{}", u, n),
            Term::String(s) => write!(f, "\"{}\"", s),
            Term::SrcPos(_, t) => t.pretty_print(f, indent),
        }
    }

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