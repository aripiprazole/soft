use std::{
    collections::VecDeque,
    convert::Infallible,
    ops::{ControlFlow, FromResidual, Try},
    sync::{Arc, RwLock},
};

use Trampoline::{Continue, Done, Raise};

use crate::{
    semantic::{self, defmacro::keyword, Expr},
    SrcPos,
};

pub struct Frame {
    pub name: Option<String>,
    pub src_pos: SrcPos,
    pub variables: im::HashMap<String, Expr>,
    pub is_catching_scope: bool,
}

#[derive(Clone)]
pub enum Value {
    Fun(semantic::Fun),
    List(Arc<Vec<Value>>),
    Literal(semantic::Literal),
    Apply(semantic::Apply),
    Def(semantic::Def),
    DefMacro(semantic::DefMacro),
    Quote(semantic::Quote),
    Atomic(Arc<RwLock<Value>>),
    Ptr(*mut ()),
    Nil,
}

pub struct Environment {
    pub global: Value,
    pub expanded: bool,
    pub frames: Arc<VecDeque<Frame>>,
}

pub enum Trampoline<T, E = Expr> {
    Done(T),
    Raise(E),
    Continue(Box<dyn Fn() -> Trampoline<T>>),
}

pub fn eval(expr: Expr, environment: Environment) -> Trampoline<Value> {
    match expr {
        Expr::List(_) => todo!(),
        Expr::Apply(_) => todo!(),
        Expr::Def(_) => todo!(),
        Expr::DefMacro(_) => todo!(),
        Expr::Quote(_) => todo!(),
        Expr::Recur(_) => todo!(),
        Expr::Deref(deref) => {
            let Value::Atomic(atomic) = eval(deref.value()?, environment)? else {
                bail!(keyword!("eval.error/atomic-expected"))
            };
            let guard = atomic.read().expect("poisoned atomic");

            Done(guard.clone())
        }
        Expr::Atomic(_) => todo!(),
        Expr::Set(_) => todo!(),

        // Bridges one-to-one from Value, to Expr, and back to Value.
        Expr::Fun(fun) => Trampoline::Done(Value::Fun(fun)),
        Expr::Literal(literal) => Trampoline::Done(Value::Literal(literal)),
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
