use std::{
    convert::Infallible,
    ops::{ControlFlow, FromResidual, Try},
    sync::{Arc, RwLock},
};

use im::HashMap;
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
    pub name: Option<Expr>,
    pub src_pos: SrcPos,
    pub definitions: im::HashMap<Keyword, Definition>,
    pub is_catching_scope: bool,
}

/// Closure function.
#[derive(Clone)]
pub struct Fun {
    pub name: Expr,
    pub parameters: Vec<Keyword>,
    pub body: Expr,
    pub environment: Arc<Environment>,
}

/// Bail out of the current evaluation with an error.
macro_rules! bail {
    ($expr:expr) => {
        return $crate::eval::Trampoline::Raise($expr.into())
    };
}

/// A value in the language. It's the lowest level of representation of a
/// value, and is used for both the AST and the runtime.
#[derive(Clone, Default)]
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
    Def(Keyword, Box<Value>),
    DefMacro(Keyword, Box<Value>),
    Recur(Vec<Value>),
    Quote(Expr),
    Ptr(*mut ()),

    #[default]
    Nil,
}

impl Value {
    /// Reads the values into S-Expressions again
    pub fn readback(self) -> Term {
        todo!()
    }
}

#[derive(Debug, Hash, Clone, PartialEq, Eq)]
pub struct Keyword {
    pub text: String,
    pub is_atom: bool,
}

impl Keyword {
    pub fn is_keyword(&self, name: &str) -> bool {
        self.text == name
    }
}

/// The environment in which evaluation takes place.
#[derive(Clone, Default)]
pub struct Environment {
    pub global: Value,
    pub expanded: bool,
    pub frames: Arc<RwLock<im::Vector<Frame>>>,
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

/// Errors that can occur during evaluation.
#[derive(Error, Debug, Clone)]
pub enum EvalError {
    #[error("undefined keyword")]
    UndefinedKeyword(Keyword),

    #[error("expected fun")]
    ExpectedFun,

    #[error("expected atomic")]
    ExpectedAtomic,

    #[error("incorrect arity")]
    IncorrectArity,
}

impl From<EvalError> for Expr {
    fn from(value: EvalError) -> Self {
        match value {
            EvalError::UndefinedKeyword(Keyword { text: name, .. }) => {
                soft_vec!(keyword!("eval.error/expected-keyword"), name)
            }
            EvalError::ExpectedFun => keyword!("eval.error/expected-fun"),
            EvalError::ExpectedAtomic => keyword!("eval.error/expected-atomic"),
            EvalError::IncorrectArity => keyword!("eval.error/incorrect-arity"),
        }
    }
}

/// A trampoline for evaluation. It's treated like a result, but it can also
/// contain a continuation.
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

impl Frame {
    /// Set a definition in the frame.
    pub fn insert_definition(&mut self, name: impl Into<Keyword>, value: Value) {
        let keyword: Keyword = name.into();
        self.definitions.insert(keyword.clone(), Definition {
            is_macro_definition: false,
            name: keyword.text,
            value,
        });
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

    /// Add frame to the environment.
    pub fn push_frame(&self, name: Expr, src_pos: SrcPos) {
        self.frames.write().unwrap().push_back(Frame {
            src_pos,
            name: Some(name),
            definitions: im::HashMap::new(),
            is_catching_scope: false,
        });
    }
}

/// Associate parameters with arguments.
fn associate_parameters(
    mut parameters: Vec<Keyword>,
    mut arguments: Vec<Value>,
) -> Trampoline<HashMap<Keyword, Value>> {
    // last two vararg & and the name
    let len = parameters.len();
    let mut environment = im::HashMap::new();
    let vararg_parameter = if len > 2 && parameters[len - 2].is_keyword("&") {
        parameters.remove(parameters.len() - 2); // remove &
        Some(parameters[parameters.len() - 1].clone())
    } else {
        None
    };

    for (index, parameter) in parameters.iter().enumerate() {
        match (arguments.first(), vararg_parameter.clone()) {
            (Some(_), Some(ref parameter)) if index == parameters.len() - 1 => {
                environment.insert(parameter.clone(), Value::List(arguments));
                break;
            }
            (None, _) => bail!(EvalError::IncorrectArity),
            (Some(argument), _) => environment.insert(parameter.clone(), argument.clone()),
        };

        arguments.remove(0);
    }

    Done(environment)
}

impl Fun {
    /// Call the function.
    pub fn call(&self, environment: &Environment, arguments: Vec<Value>) -> Trampoline<Value> {
        environment.push_frame(self.name.clone(), SrcPos::default());

        let mut current_environment = self.environment.frames.write().unwrap();
        let frame = current_environment.back_mut().unwrap();
        for (name, value) in associate_parameters(self.parameters.clone(), arguments.clone())? {
            frame.definitions.insert(name.clone(), Definition {
                is_macro_definition: false,
                name: name.text,
                value,
            });
        }

        self.body.clone().expand(environment)?.eval(environment)
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
            _ => Ok(Value::Apply {
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
        name: fun.name()?,
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
        environment: Arc::new(environment.clone()),
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
                /* elements: */
                list.elements()?
                    .into_iter()
                    .map(|expr| expr.expand(environment))
                    .collect::<Result<Vec<_>, _>>()?,
            )),
            Expr::Def(def) => Ok(Value::Def(
                /* name : */ def.name()?.expand(environment)?.try_into()?,
                /* value: */ def.value()?.expand(environment)?.into(),
            )),
            Expr::Recur(recur) => Ok(Value::Recur(
                /* arguments: */
                recur
                    .spine()?
                    .into_iter()
                    .map(|expr| expr.expand(environment))
                    .collect::<Result<Vec<_>, _>>()?,
            )),
            Expr::DefMacro(def_macro) => Ok(Value::DefMacro(
                /* name : */ def_macro.name()?.expand(environment)?.try_into()?,
                /* value: */ def_macro.value()?.expand(environment)?.into(),
            )),

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
                    text: n.clone(),
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
            text: name,
            is_atom: false,
        }
    }
}

impl From<&str> for Keyword {
    fn from(name: &str) -> Self {
        Self {
            text: name.to_string(),
            is_atom: false,
        }
    }
}
