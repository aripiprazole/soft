use std::{
    convert::Infallible,
    ops::{ControlFlow, FromResidual, Try},
    sync::{Arc, RwLock},
};

use Trampoline::{Continue, Done, Raise};

use crate::{
    semantic::{self, defmacro::keyword, Expr, Literal},
    SrcPos, Term,
};

pub struct Definition {
    pub is_macro_definition: bool,
    pub name: String,
    pub value: Value,
}

pub struct Frame {
    pub name: Option<String>,
    pub src_pos: SrcPos,
    pub definitions: im::HashMap<String, Expr>,
    pub is_catching_scope: bool,
}

#[derive(Clone)]
pub struct Keyword {
    pub name: String,
}

#[derive(Clone)]
pub enum Value {
    Fun(semantic::Fun),
    List(Arc<Vec<Value>>),
    Int(u64),
    Keyword(Keyword),
    String(String),
    Float(u64),
    Apply {
        callee: Box<Value>,
        arguments: Vec<Value>,
    },
    Def {
        name: Keyword,
        value: Box<Value>,
    },
    DefMacro {
        name: Keyword,
        value: Box<Value>,
    },
    Set {
        target: Box<Value>,
        value: Box<Value>,
    },
    Deref {
        value: Box<Value>,
    },
    Recur {
        arguments: Vec<Value>,
    },
    Quote(Expr),
    Atomic(Arc<RwLock<Value>>),
    Ptr(*mut ()),
    Nil,
}

pub struct Environment {
    pub global: Value,
    pub expanded: bool,
    pub frames: Arc<RwLock<im::Vector<Frame>>>,
}

pub enum Trampoline<T, E = Expr> {
    Done(T),
    Raise(E),
    Continue(Box<dyn Fn() -> Trampoline<T>>),
}

impl Trampoline<Value> {
    pub fn eval_into_result(self) -> Result<Value, Expr> {
        match self.branch() {
            ControlFlow::Continue(value) => Ok(value),
            ControlFlow::Break(Err(err)) => Err(err),
            _ => unreachable!(),
        }
    }
}

impl Expr {
    pub fn expand(self, environment: &Environment) -> Result<Value, Expr> {
        match self {
            Expr::Fun(_) => todo!(),
            Expr::List(_) => todo!(),
            Expr::Apply(_) => todo!(),
            Expr::Def(_) => todo!(),
            Expr::Recur(recur) => Ok(Value::Recur {
                arguments: recur
                    .spine()?
                    .into_iter()
                    .map(|expr| expr.expand(environment))
                    .collect::<Result<Vec<_>, _>>()?,
            }),
            Expr::Deref(deref) => Ok(Value::Deref {
                value: deref.value()?.expand(environment)?.into(),
            }),
            Expr::Atomic(_) => todo!(),
            Expr::Set(_) => todo!(),
            Expr::DefMacro(_) => todo!(),
            Expr::Quote(expr) => Ok(Value::Quote(expr.expression()?)),
            Expr::Literal(Literal(Term::Atom(keyword))) => Ok(Value::Keyword(Keyword {
                name: keyword,
            })),
            Expr::Literal(Literal(Term::Identifier(identifier))) => Ok(Value::Keyword(Keyword {
                name: identifier,
            })),
            Expr::Literal(Literal(Term::Int(value))) => Ok(Value::Int(value)),
            Expr::Literal(Literal(Term::String(value))) => Ok(Value::String(value)),
            Expr::Literal(Literal(Term::Float(_, _))) => todo!(),
            Expr::Literal(_) => Err(keyword!("eval.error/invalid-literal")),
        }
    }

    pub fn eval(self, environment: &Environment) -> Trampoline<Value> {
        match self.expand(environment)? {
            Value::Deref { box value, .. } => {
                let Value::Atomic(atomic) = value else {
                    bail!(keyword!("eval.error/atomic-expected"))
                };
                let guard = atomic.read().expect("poisoned atomic");

                Done(guard.clone())
            }

            value => Done(value),
        }
    }
}

impl<T> Try for Trampoline<T, Expr> {
    type Output = T;
    type Residual = Result<Infallible, Expr>;

    fn from_output(output: Self::Output) -> Self {
        Done(output)
    }

    fn branch(self) -> ControlFlow<Self::Residual, Self::Output> {
        let mut value: Trampoline<T, Expr> = self;
        loop {
            match value {
                Done(done) => return ControlFlow::Continue(done),
                Raise(error) => return ControlFlow::Break(Err(error)),
                Continue(f) => {
                    value = f();
                }
            }
        }
    }
}

impl<T, E, F: From<E>> FromResidual<Result<Infallible, E>> for Trampoline<T, F> {
    fn from_residual(residual: Result<Infallible, E>) -> Self {
        match residual {
            Err(error) => Raise(From::from(error)),
            _ => unreachable!(),
        }
    }
}

macro_rules! bail {
    ($expr:expr) => {
        return $crate::eval::Trampoline::Raise($expr.into())
    };
}

use bail;
