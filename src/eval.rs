use std::{
    convert::Infallible,
    ops::{ControlFlow, FromResidual, Try},
    sync::{Arc, RwLock},
};

use thiserror::Error;
use Trampoline::{Continue, Done, Raise};

use crate::{keyword, soft_vec, Expr, Literal, SrcPos, Term};

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

/// Closure function.
#[derive(Clone)]
pub struct Fun {
    pub parameters: Vec<Keyword>,
    pub body: Expr,
}

/// Bail out of the current evaluation with an error.
macro_rules! bail {
    ($expr:expr) => {
        return $crate::eval::Trampoline::Raise($expr.into())
    };
}

/// A value in the language. It's the lowest level of representation of a
/// value, and is used for both the AST and the runtime.
#[derive(Clone)]
pub enum Value {
    Int(u64),
    Keyword(Keyword),
    String(String),
    Float(u64),
    Fun(Fun),
    List(Vec<Value>),
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
    Recur {
        arguments: Vec<Value>,
    },
    Quote(Expr),
    Ptr(*mut ()),
    Nil,
}

#[derive(Debug, Hash, Clone, PartialEq, Eq)]
pub struct Keyword {
    pub name: String,
    pub is_atom: bool,
}

/// The environment in which evaluation takes place.
pub struct Environment {
    pub global: Value,
    pub expanded: bool,
    pub frames: Arc<RwLock<im::Vector<Frame>>>,
}

/// Errors that can occur during evaluation.
#[derive(Error, Debug, Clone)]
pub enum EvalError {
    #[error("undefined keyword")]
    UndefinedKeyword(Keyword),

    #[error("expected fun")]
    ExpectedFun,

    #[error("expected atomic")]
    ExpectedAtomic,
}

impl From<EvalError> for Expr {
    fn from(value: EvalError) -> Self {
        match value {
            EvalError::UndefinedKeyword(Keyword { name, .. }) => {
                soft_vec!(keyword!("eval.error/expected-keyword"), name)
            }
            EvalError::ExpectedFun => keyword!("eval.error/expected-fun"),
            EvalError::ExpectedAtomic => keyword!("eval.error/expected-atomic"),
        }
    }
}

/// Errors that can occur during expansion.
#[derive(Error, Debug, Clone)]
pub enum ExpansionError {
    #[error("expected keyword")]
    ExpectedKeyword,
}

impl From<ExpansionError> for Expr {
    fn from(error: ExpansionError) -> Self {
        match error {
            ExpansionError::ExpectedKeyword => keyword!("eval.error/expected-keyword"),
        }
    }
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
    type Error = ExpansionError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::Keyword(keyword) => Ok(keyword),
            _ => Err(ExpansionError::ExpectedKeyword),
        }
    }
}

impl Fun {
    /// Call the function.
    pub fn call(&self, environment: &Environment, arguments: Vec<Value>) -> Trampoline<Value> {
        let _ = environment;
        let _ = arguments;
        todo!()
    }
}

/// Expand apply expressions.
fn apply_expand(apply: crate::Apply, environment: &Environment) -> Result<Value, Expr> {
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

/// Expand fun expressions.
fn fun_expand(fun: crate::Fun, environment: &Environment) -> Result<Value, Expr> {
    Ok(Value::Fun(Fun {
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
    }))
}

impl Expr {
    /// Expand the expression into a value.
    pub fn expand(self, environment: &Environment) -> Result<Value, Expr> {
        match self {
            Expr::Apply(apply) => apply_expand(apply, environment),
            Expr::Fun(fun) => fun_expand(fun, environment),

            // Base cases for expansion when it will just walk the tree. These
            // are the cases where the expansion is recursive.
            Expr::List(list) => Ok(Value::List(
                list.elements()?
                    .into_iter()
                    .map(|expr| expr.expand(environment))
                    .collect::<Result<Vec<_>, _>>()?,
            )),
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
            Expr::DefMacro(def_macro) => Ok(Value::DefMacro {
                name: def_macro.name()?.expand(environment)?.try_into()?,
                value: def_macro.value()?.expand(environment)?.into(),
            }),

            // Expansion of literal terms, just wrap them in a value. This is
            // the base case of the expansion.
            Expr::Quote(expr) => Ok(Value::Quote(expr.expr()?)),
            Expr::Literal(Literal(Term::Int(value))) => Ok(Value::Int(value)),
            Expr::Literal(Literal(Term::String(value))) => Ok(Value::String(value)),
            Expr::Literal(Literal(Term::Float(_, _))) => todo!(),
            Expr::Literal(Literal(ref t @ Term::Identifier(ref n) | ref t @ Term::Atom(ref n))) => {
                if let Some(definition) = environment.find_definition(n.clone()) {
                    if definition.is_macro_definition {
                        return Ok(definition.value.clone());
                    }
                }

                Ok(Value::Keyword(Keyword {
                    name: n.clone(),
                    is_atom: matches!(t, Term::Atom(_)),
                }))
            }
            Expr::Literal(_) => Err(keyword!("eval.error/invalid-literal")),
        }
    }
}

impl Value {
    /// Evaluate the expression into a value.
    pub fn eval(self, environment: &Environment) -> Trampoline<Value> {
        match self {
            Value::Keyword(keyword) if !keyword.is_atom => {
                match environment.find_definition(keyword.clone()) {
                    Some(Definition { value, .. }) => Done(value),
                    None => bail!(EvalError::UndefinedKeyword(keyword)),
                }
            }
            Value::Apply { callee, arguments } => match callee.eval(environment)? {
                Value::Fun(fun) => {
                    let mut new_arguments = Vec::new();
                    for argument in arguments {
                        new_arguments.push(argument.eval(environment)?);
                    }
                    fun.call(environment, new_arguments)
                }
                _ => bail!(EvalError::ExpectedFun),
            },
            Value::List(old_elements) => {
                let mut new_elements = Vec::new();
                for element in old_elements {
                    new_elements.push(element.eval(environment)?);
                }

                Done(Value::List(new_elements))
            }
            Value::DefMacro { .. } | Value::Def { .. } => Done(Value::Nil),

            // Base cases for evaluation when it will just walk the tree. These
            // are the cases where the evaluation is recursive.
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

impl From<String> for Keyword {
    fn from(name: String) -> Self {
        Self {
            name,
            is_atom: false,
        }
    }
}

impl From<&str> for Keyword {
    fn from(name: &str) -> Self {
        Self {
            name: name.to_string(),
            is_atom: false,
        }
    }
}
