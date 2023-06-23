//! This module defines the values that are used by the interpreter.

use crate::environment::{Environment, Frame};
use crate::error::{Result, RuntimeError};

use std::fmt::Debug;
use std::{
    cell::UnsafeCell,
    fmt::Display,
    ops::{Deref, DerefMut},
    rc::Rc,
};

#[derive(Debug)]
pub enum Trampoline {
    Eval(Value),
    Return(Value),
}

impl Display for Trampoline {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Trampoline::Eval(value) => write!(f, "Eval({})", value),
            Trampoline::Return(value) => write!(f, "Return({})", value),
        }
    }
}

/// A location is a span of text in a file.
#[derive(Clone, Debug)]
pub struct Location {
    pub line: usize,
    pub column: usize,
    pub file: Option<String>,
}

impl Display for Location {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(file) = &self.file {
            write!(f, "{}:", file)?;
        }
        write!(f, "{}:{}", self.line, self.column)
    }
}

/// A value is a type that can be used by the interpreter.
#[derive(Debug)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Option<Location>,
}

impl Expr {
    pub fn new(kind: ExprKind, span: Option<Location>) -> Self {
        Self { kind, span }
    }
}

/// A scope for a function call.
pub struct CallScope<'a> {
    pub args: Vec<Value>,
    pub env: &'a mut Environment,
    pub location: Option<Location>,
}

impl CallScope<'_> {
    pub fn at(&self, nth: usize) -> Value {
        self.args
            .get(nth)
            .cloned()
            .unwrap_or_else(|| ExprKind::Nil.to_value())
    }

    pub fn assert_arity(&self, size: usize) -> Result<()> {
        if self.args.len() != size {
            Err(RuntimeError::WrongArity(size, self.args.len()))
        } else {
            Ok(())
        }
    }

    pub fn assert_at_least(&self, size: usize) -> Result<()> {
        if self.args.len() < size {
            Err(RuntimeError::WrongArity(size, self.args.len()))
        } else {
            Ok(())
        }
    }
}

pub type Prim = fn(CallScope<'_>) -> Result<Trampoline>;

/// External functions that can be called from the interpreter.
#[derive(Clone)]
pub struct Extern {
    pub name: &'static str,
    pub call: Prim,
}

impl Debug for Extern {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Extern").finish()
    }
}

/// A closure is a function that can be called from the interpreter.
#[derive(Debug, Clone)]
pub struct Closure {
    pub name: Option<String>,
    pub frame: Frame,
    pub params: Vec<String>,
    pub expr: Value,
}

#[derive(Debug)]
pub enum Function {
    Extern(Prim),
    Closure(Closure),
}

/// An expression is a value that can be evaluated to produce another expression.
#[derive(Debug)]
pub enum ExprKind {
    Int(i64),
    Id(String),
    Str(String),
    Cons(Value, Value),
    Function(Function),
    Nil,
}

impl ExprKind {
    pub fn to_value(self) -> Value {
        Expr::new(self, None).into()
    }

    pub fn is_nil(&self) -> bool {
        matches!(self, ExprKind::Nil)
    }

    pub fn is_cons(&self) -> bool {
        matches!(self, ExprKind::Cons(_, _))
    }
}

/// A value is a reference-counted, mutable expression that is not save to share but is used
/// internally by the runtime.
#[derive(Clone, Debug)]
pub struct Value(Rc<UnsafeCell<Expr>>);

impl Value {
    pub fn is_true(&self) -> bool {
        !self.is_nil()
    }

    pub fn assert_identifier(&self) -> Result<String> {
        match self.kind {
            ExprKind::Id(ref id) => Ok(id.clone()),
            _ => Err(RuntimeError::ExpectedIdentifier(self.to_string())),
        }
    }

    pub fn assert_list(&self) -> Result<Vec<Value>> {
        let (list, tail) = self
            .to_list()
            .ok_or_else(|| RuntimeError::ExpectedList(self.to_string()))?;

        if tail.is_some() {
            return Err(RuntimeError::ExpectedList(self.to_string()));
        }

        Ok(list)
    }

    pub fn assert_number(&self) -> Result<i64> {
        match self.kind {
            ExprKind::Int(ref int) => Ok(*int),
            _ => Err(RuntimeError::ExpectedNumber(self.to_string())),
        }
    }
}

impl Value {
    pub fn from_iter<I>(iter: I, location: Option<Location>) -> Self
    where
        I: DoubleEndedIterator<Item = Self>,
    {
        let iter = iter.into_iter().rev();
        let mut value = Expr::new(ExprKind::Nil, location).into();

        for next in iter {
            let span = next.span.clone();
            value = Expr::new(ExprKind::Cons(next, value), span).into();
        }

        value
    }

    pub fn is_nil(&self) -> bool {
        self.kind.is_nil()
    }

    pub fn to_list(&self) -> Option<(Vec<Value>, Option<Value>)> {
        let mut list = Vec::new();
        let mut value = self.clone();

        if !value.kind.is_cons() {
            return None;
        }

        while !value.is_nil() {
            match value.kind {
                ExprKind::Cons(ref head, ref tail) => {
                    list.push(head.clone());
                    value = tail.clone();
                }
                _ => return Some((list, Some(value))),
            }
        }

        Some((list, None))
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.kind {
            ExprKind::Int(int) => write!(f, "{}", int),
            ExprKind::Id(ref id) => write!(f, "{}", id),
            ExprKind::Str(ref string) => write!(f, "\"{}\"", string),
            ExprKind::Cons(..) => {
                let (list, not_nil) = self.to_list().unwrap();
                write!(f, "(")?;
                if !list.is_empty() {
                    write!(f, "{}", list[0])?;
                    for item in &list[1..] {
                        write!(f, " {}", item)?;
                    }
                }
                if let Some(not_nil) = not_nil {
                    write!(f, " . {}", not_nil)?;
                }
                write!(f, ")")
            }
            ExprKind::Nil => write!(f, "()"),
            ExprKind::Function(..) => write!(f, "<function>"),
        }
    }
}

impl From<Expr> for Value {
    fn from(expr: Expr) -> Self {
        Self(Rc::new(UnsafeCell::new(expr)))
    }
}

impl Deref for Value {
    type Target = Expr;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.0.get() }
    }
}

impl DerefMut for Value {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.0.get() }
    }
}
