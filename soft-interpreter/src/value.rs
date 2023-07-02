//! This module defines the values that are used by the interpreter.

use fxhash::FxHashMap;

use crate::environment::{Environment, Frame};
use crate::error::{Result, RuntimeError};

use core::fmt;
use std::fmt::{Debug, Display, Formatter};
use std::{
    cell::UnsafeCell,
    ops::{Deref, DerefMut},
    rc::Rc,
};

#[derive(Debug)]
pub enum Trampoline {
    Eval(Value),
    Return(Value),
}

impl Trampoline {
    pub fn returning<I: Into<Value>>(expr: I) -> Trampoline {
        let expr: Value = expr.into();
        Trampoline::Return(expr)
    }

    pub fn eval<I: Into<Value>>(expr: I) -> Trampoline {
        let expr: Value = expr.into();
        Trampoline::Eval(expr)
    }
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
#[derive(Debug, Clone)]
pub struct Spanned<T> {
    pub kind: T,
    pub span: Option<Location>,
}

impl<T> Spanned<T> {
    pub fn new(kind: T, span: Option<Location>) -> Self {
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
            .unwrap_or_else(|| Expr::Nil.to_value())
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

pub type External = fn(*const u64) -> Result<Trampoline>;

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

#[derive(Debug, Clone)]
pub enum Param {
    Required(String),
    Optional(String, Value),
    Variadic(String),
}

/// A closure is a function that can be called from the interpreter.
#[derive(Debug, Clone)]
pub struct Closure {
    pub name: Option<String>,
    pub frame: Frame,
    pub params: Vec<Param>,
    pub location: Option<Location>,
    pub expr: Value,
}

#[derive(Debug, Clone)]
pub enum Function {
    Extern(Prim),
    Closure(Closure),
}

#[derive(Debug, Clone)]
pub enum Type {
    Int,
    String,
}

/// An expression is a value that can be evaluated to produce another expression.
#[derive(Debug, Clone)]
pub enum Expr {
    Int(i64),
    Id(String),
    Str(String),
    Cons(Value, Value),
    Atom(String),
    Function(Function),
    Err(RuntimeError, Vec<Frame>),
    Vector(Vec<Value>),
    HashMap(FxHashMap<String, (Value, Value)>),
    Library(*mut libc::c_void),
    External(*mut libc::c_void, Vec<Type>),
    Nil,
}

impl Expr {
    pub fn to_value(self) -> Value {
        Spanned::new(self, None).into()
    }

    pub fn is_nil(&self) -> bool {
        matches!(self, Expr::Nil)
    }

    pub fn is_cons(&self) -> bool {
        matches!(self, Expr::Cons(_, _))
    }
}

/// A value is a reference-counted, mutable expression that is not save to share but is used
/// internally by the runtime.
#[derive(Clone, Debug)]
pub struct Value(Rc<UnsafeCell<Spanned<Expr>>>);

impl Value {
    pub fn quote(self) -> Value {
        Expr::Cons(
            Expr::Id("quote".to_string()).to_value(),
            Expr::Cons(self, Expr::Nil.into()).to_value(),
        )
        .into()
    }

    pub fn is_cons(self) -> bool {
        self.borrow_mut().kind.is_cons()
    }

    pub fn stringify(&self) -> String {
        match self.kind {
            Expr::Str(ref string) => string.clone(),
            _ => self.to_string(),
        }
    }

    pub fn borrow_mut<'a>(self) -> &'a mut Spanned<Expr> {
        unsafe { &mut *self.0.get() }
    }

    pub fn is_true(&self) -> bool {
        !self.is_nil()
    }

    pub fn assert_string(&self) -> Result<String> {
        match self.kind {
            Expr::Str(ref string) => Ok(string.clone()),
            _ => Err(RuntimeError::ExpectedString(self.to_string())),
        }
    }

    pub fn assert_library(&self) -> Result<*mut libc::c_void> {
        match self.kind {
            Expr::Library(lib) => Ok(lib),
            _ => Err(RuntimeError::UserError(
                Expr::Str(format!("expected library, got {}", self.to_string())).into(),
            )),
        }
    }

    pub fn assert_error(&self) -> Result<(RuntimeError, Vec<Frame>)> {
        match self.kind {
            Expr::Err(ref err, ref stack) => Ok((err.clone(), stack.clone())),
            _ => Err(RuntimeError::ExpectedErr(self.to_string())),
        }
    }

    pub fn assert_identifier(&self) -> Result<String> {
        match self.kind {
            Expr::Id(ref id) => Ok(id.clone()),
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

    pub fn assert_external(&self) -> Result<(*mut libc::c_void, Vec<Type>)> {
        match &self.kind {
            Expr::External(ptr, ty) => Ok((*ptr, ty.clone())),
            _ => Err(RuntimeError::UserError(
                Expr::Str(format!("expected external, got {}", self.to_string())).into(),
            )),
        }
    }

    pub fn assert_vector(&self) -> Result<Vec<Value>> {
        match self.kind {
            Expr::Vector(ref vector) => Ok(vector.clone()),
            _ => Err(RuntimeError::UserError(
                Expr::Str(format!("expected vector, got {}", self.to_string())).into(),
            )),
        }
    }

    pub fn assert_atom(&self) -> Result<String> {
        match self.kind {
            Expr::Atom(ref atom) => Ok(atom.clone()),
            _ => Err(RuntimeError::UserError(
                Expr::Str(format!("expected atom, got {}", self.to_string())).into(),
            )),
        }
    }

    pub fn assert_fixed_size_list(&self, size: usize) -> Result<Vec<Value>> {
        let (list, tail) = self
            .to_list()
            .ok_or_else(|| RuntimeError::ExpectedList(self.to_string()))?;

        if tail.is_some() {
            return Err(RuntimeError::ExpectedList(self.to_string()));
        }

        if list.len() != size {
            return Err(RuntimeError::WrongArity(2, list.len()));
        }

        Ok(list)
    }

    pub fn assert_number(&self) -> Result<i64> {
        match self.kind {
            Expr::Int(ref int) => Ok(*int),
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
        let mut value = Spanned::new(Expr::Nil, location).into();

        for next in iter {
            let span = next.span.clone();
            value = Spanned::new(Expr::Cons(next, value), span).into();
        }

        value
    }

    pub fn to_bool(&self) -> bool {
        match &self.kind {
            Expr::Nil => false,
            Expr::Atom(x) if x == "false" => false,
            _ => true,
        }
    }

    pub fn from_bool(cond: bool) -> Value {
        if cond {
            Expr::Id("true".to_string()).into()
        } else {
            Expr::Nil.into()
        }
    }

    pub fn is_nil(&self) -> bool {
        self.kind.is_nil()
    }

    pub fn is_vec(&self) -> bool {
        matches!(self.kind, Expr::Vector(_))
    }

    pub fn is_function(&self) -> bool {
        matches!(self.kind, Expr::Function(_))
    }

    pub fn is_error(&self) -> bool {
        matches!(self.kind, Expr::Err(_, _))
    }

    pub fn is_int(&self) -> bool {
        matches!(self.kind, Expr::Int(_))
    }

    pub fn is_id(&self) -> bool {
        matches!(self.kind, Expr::Id(_))
    }

    pub fn is_atom(&self) -> bool {
        matches!(self.kind, Expr::Atom(_))
    }

    pub fn is_str(&self) -> bool {
        matches!(self.kind, Expr::Str(_))
    }

    pub fn to_list(&self) -> Option<(Vec<Value>, Option<Value>)> {
        let mut list = Vec::new();
        let mut value = self.clone();

        if !value.kind.is_cons() && !value.is_nil() {
            return None;
        }

        while !value.is_nil() {
            match value.kind {
                Expr::Cons(ref head, ref tail) => {
                    list.push(head.clone());
                    value = tail.clone();
                }
                _ => return Some((list, Some(value))),
            }
        }

        Some((list, None))
    }
}

pub enum SExpr {
    Label(String),
    List(Vec<SExpr>),
}

impl SExpr {
    fn text_width(&self) -> usize {
        match self {
            SExpr::Label(label) => label.len(),
            SExpr::List(list) => {
                let mut width = 2;

                for item in list {
                    width += item.text_width();
                }

                width
            }
        }
    }

    fn pretty_print(&self, f: &mut Formatter<'_>, indent: usize, first: bool) -> fmt::Result {
        let width = self.text_width();

        if !first {
            write!(f, "{}", " ".repeat(indent))?;
        }

        match self {
            SExpr::Label(label) => write!(f, "{}", label),
            SExpr::List(list) if width + indent > 80 => {
                write!(f, "(")?;

                for (i, item) in list.iter().enumerate() {
                    item.pretty_print(f, indent + 2, i == 0)?;

                    if i != list.len() - 1 {
                        write!(f, "\n")?;
                    }
                }

                write!(f, ")")
            }
            Self::List(list) => {
                write!(f, "(")?;

                for (i, item) in list.iter().enumerate() {
                    if i != 0 {
                        write!(f, " ")?;
                    }
                    item.pretty_print(f, 0, true)?;
                }

                write!(f, ")")
            }
        }
    }
}

impl Display for SExpr {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.pretty_print(f, 0, false)
    }
}

impl From<Value> for SExpr {
    fn from(value: Value) -> Self {
        match &value.kind {
            Expr::Cons(_, _) => {
                let (list, not_nil) = value.to_list().unwrap();

                let mut list = list.into_iter().map(|x| x.into()).collect::<Vec<_>>();

                if let Some(not_nil) = not_nil {
                    list.push(SExpr::Label(".".to_string()));
                    list.push(not_nil.into());
                }

                SExpr::List(list)
            }
            Expr::Vector(vector) => {
                let mut vec: Vec<_> = vector.iter().map(|x| x.clone().into()).collect();
                vec.insert(0, SExpr::Label("vec".to_string()));
                SExpr::List(vec)
            }
            Expr::HashMap(values) => {
                let mut list = Vec::new();

                for (key, value) in values.values() {
                    list.push(SExpr::List(vec![key.clone().into(), value.clone().into()]));
                }

                list.insert(0, SExpr::Label("hash-map".to_string()));

                SExpr::List(list)
            }

            Expr::Id(id) => SExpr::Label(id.to_string()),
            Expr::Nil => SExpr::List(Vec::new()),
            Expr::Int(int) => SExpr::Label(int.to_string()),
            Expr::Str(string) => SExpr::Label(format!("\"{}\"", string)),
            Expr::Function(..) => SExpr::Label("<function>".to_string()),
            Expr::Err(string, _) => SExpr::Label(string.to_string()),
            Expr::Atom(atom) => SExpr::Label(format!(":{}", atom)),
            Expr::External(..) => SExpr::Label("<external>".to_string()),
            Expr::Library(_) => todo!(),
        }
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        SExpr::from(self.clone()).fmt(f)
    }
}

impl From<Expr> for Value {
    fn from(value: Expr) -> Self {
        Spanned::new(value, None).into()
    }
}

impl From<Expr> for Spanned<Expr> {
    fn from(value: Expr) -> Self {
        Spanned::new(value, None)
    }
}

impl From<Spanned<Expr>> for Value {
    fn from(expr: Spanned<Expr>) -> Self {
        Self(Rc::new(UnsafeCell::new(expr)))
    }
}

impl Deref for Value {
    type Target = Spanned<Expr>;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.0.get() }
    }
}

impl DerefMut for Value {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.0.get() }
    }
}
