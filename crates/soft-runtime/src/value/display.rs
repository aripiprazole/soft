use std::fmt::Display;

use super::*;

impl Value {
    fn accumulate_cons(&self) -> (Vec<Value>, Option<Value>) {
        let mut acc = vec![];
        let mut cur = *self;

        loop {
            match cur.classify() {
                FatPtr::Cons(cons) => {
                    let cons = cons.untag();
                    acc.push(cons.head);
                    cur = cons.tail;
                }
                FatPtr::Nil => return (acc, None),
                _ => return (acc, Some(cur)),
            }
        }
    }
}

impl Display for Cons {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (acc, tail) = self.tail.accumulate_cons();

        write!(f, "({}", self.head)?;
        write!(
            f,
            "{}",
            acc.iter()
                .map(|x| format!(" {}", x))
                .collect::<Vec<_>>()
                .join("")
        )?;

        if let Some(res) = tail {
            write!(f, " . {}", res)?;
        }

        write!(f, ")")
    }
}

impl Display for Vector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[")?;
        if self.size > 0 {
            write!(f, "{}", self.get(0))?;
            for i in 1..self.size {
                let el = self.get(i);
                write!(f, " {}", el)?;
            }
        }
        write!(f, "]")
    }
}

impl Display for Str {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "\"{}\"", self.0)
    }
}

impl Display for Symbol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#<symbol>")
    }
}

impl Display for Closure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#<closure>")
    }
}

impl Display for Int {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Display for Char {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "'{}'", char::from_u32(self.0).unwrap())
    }
}

impl Display for Bool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Bool::True => write!(f, "true"),
            Bool::False => write!(f, "false"),
        }
    }
}

impl Display for Nil {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "nil")
    }
}

impl Display for FatPtr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FatPtr::Cons(cons) => write!(f, "{}", cons.untag()),
            FatPtr::Vector(vector) => write!(f, "{}", vector.untag()),
            FatPtr::Str(string) => write!(f, "{}", string.untag()),
            FatPtr::Symbol(symbol) => write!(f, "{}", symbol.untag()),
            FatPtr::Closure(closure) => write!(f, "{}", closure.untag()),
            FatPtr::Int(int) => write!(f, "{}", int),
            FatPtr::Char(char) => write!(f, "{}", char),
            FatPtr::Bool(bool) => write!(f, "{}", bool),
            FatPtr::Nil => write!(f, "nil"),
        }
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", FatPtr::from(self.0))
    }
}

