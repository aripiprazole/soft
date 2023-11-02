use std::fmt::Display;

use crate::Term;

impl Term {
    pub fn width(&self) -> usize {
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

    pub fn pretty_print(&self, f: &mut std::fmt::Formatter<'_>, indent: usize)-> std::fmt::Result  {
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
}

impl Display for Term {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.pretty_print(f, 0)
    }
}