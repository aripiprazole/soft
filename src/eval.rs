use std::{
    convert::Infallible,
    ops::{ControlFlow, FromResidual, Try},
    sync::{Arc, RwLock},
};

use Trampoline::{Continue, Done, Raise};

use crate::{
    semantic::{defmacro::keyword, Expr, Literal},
    SrcPos, Term,
};

#[derive(Clone)]
pub struct Definition {
    pub is_macro_definition: bool,
    pub name: String,
    pub value: Value,
}

#[derive(Clone)]
pub struct Frame {
    pub name: Option<String>,
    pub src_pos: SrcPos,
    pub definitions: im::HashMap<Keyword, Definition>,
    pub is_catching_scope: bool,
}

#[derive(Hash, Clone, PartialEq, Eq)]
pub struct Keyword {
    pub name: String,
}

impl From<String> for Keyword {
    fn from(name: String) -> Self {
        Self { name }
    }
}

impl From<&str> for Keyword {
    fn from(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }
}

#[derive(Clone)]
pub struct Fun {
    pub parameters: Vec<Keyword>,
    pub body: Expr,
}

impl Fun {
    pub fn call(&self, environment: &Environment, arguments: Vec<Value>) -> Trampoline<Value> {
        let _ = environment;
        let _ = arguments;
        todo!()
    }
}

#[derive(Clone)]
pub enum Value {
    Int(u64),
    Keyword(Keyword),
    String(String),
    Float(u64),
    Fun(Fun),
    List {
        elements: Vec<Value>,
    },
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

impl Environment {
    /// Find a definition in the environment.
    pub fn find_definition(&self, name: impl Into<Keyword>) -> Option<Definition> {
        let name: Keyword = name.into();
        for frame in self.frames.read().unwrap().iter().rev() {
            if let Some(expr) = frame.definitions.get(&name) {
                return Some(expr.clone());
            }
        }

        None
    }
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

impl TryFrom<Value> for Keyword {
    type Error = Expr;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::Keyword(keyword) => Ok(keyword),
            _ => Err(keyword!("eval.error/keyword-expected")),
        }
    }
}

impl Expr {
    /// Expand the expression into a value.
    pub fn expand(self, environment: &Environment) -> Result<Value, Expr> {
        match self {
            Expr::Fun(fun) => Ok(Value::Fun(Fun {
                parameters: fun
                    .parameters()?
                    .elements()?
                    .into_iter()
                    .map(|expr| expr.expand(environment))
                    .collect::<Result<Vec<_>, _>>()?
                    .into_iter()
                    .map(|value| value.try_into())
                    .collect::<Result<Vec<_>, _>>()?,
                body: fun.body()?,
            })),
            Expr::List(list) => Ok(Value::List {
                elements: list
                    .elements()?
                    .into_iter()
                    .map(|expr| expr.expand(environment))
                    .collect::<Result<Vec<_>, _>>()?,
            }),
            Expr::Apply(apply) => {
                let callee = apply.callee()?;
                if let Expr::Literal(Literal(Term::Identifier(k) | Term::Atom(k))) = callee {
                    return match environment.find_definition(k.clone()) {
                        Some(Definition {
                            name: _,
                            value: Value::Fun(fun),
                            is_macro_definition,
                        }) if is_macro_definition => {
                            let arguments = apply
                                .spine()?
                                .into_iter()
                                .map(|expr| expr.expand(environment))
                                .collect::<Result<Vec<_>, _>>()?;

                            fun.call(environment, arguments).eval_into_result()
                        }
                        None | Some(_) => Ok(Value::Apply {
                            callee: Value::Keyword(Keyword::from(k.clone())).into(),
                            arguments: apply
                                .spine()?
                                .into_iter()
                                .map(|expr| expr.expand(environment))
                                .collect::<Result<Vec<_>, _>>()?,
                        }),
                    };
                }

                Ok(Value::Apply {
                    callee: callee.expand(environment)?.into(),
                    arguments: apply
                        .spine()?
                        .into_iter()
                        .map(|expr| expr.expand(environment))
                        .collect::<Result<Vec<_>, _>>()?,
                })
            }
            Expr::Def(def) => Ok(Value::Def {
                name: def.name()?.expand(environment)?.try_into()?,
                value: def.value()?.expand(environment)?.into(),
            }),
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
            Expr::Atomic(atomic) => {
                let value = atomic.value()?.expand(environment)?;
                let atomic = Arc::new(RwLock::new(value));
                Ok(Value::Atomic(atomic))
            }
            Expr::Set(set) => Ok(Value::Set {
                target: set.target()?.expand(environment)?.into(),
                value: set.value()?.expand(environment)?.into(),
            }),
            Expr::DefMacro(def_macro) => Ok(Value::DefMacro {
                name: def_macro.name()?.expand(environment)?.try_into()?,
                value: def_macro.value()?.expand(environment)?.into(),
            }),
            Expr::Quote(expr) => Ok(Value::Quote(expr.expression()?)),
            Expr::Literal(Literal(Term::Int(value))) => Ok(Value::Int(value)),
            Expr::Literal(Literal(Term::String(value))) => Ok(Value::String(value)),
            Expr::Literal(Literal(Term::Float(_, _))) => todo!(),
            Expr::Literal(Literal(Term::Identifier(name) | Term::Atom(name))) => {
                if let Some(definition) = environment.find_definition(name.clone()) {
                    if definition.is_macro_definition {
                        return Ok(definition.value.clone());
                    }
                }

                Ok(Value::Keyword(Keyword { name }))
            }
            Expr::Literal(_) => Err(keyword!("eval.error/invalid-literal")),
        }
    }

    /// Evaluate the expression into a value.
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
