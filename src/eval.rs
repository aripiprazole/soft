use std::{
    collections::VecDeque,
    convert::Infallible,
    ops::{ControlFlow, FromResidual, Try},
    sync::{Arc, RwLock},
};

use crate::{
    semantic::{self, Expr},
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
            let value = eval(deref.value()?, environment)?;

            todo!()
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
        Trampoline::Done(output)
    }

    fn branch(self) -> ControlFlow<Self::Residual, Self::Output> {
        let mut value: Trampoline<T, Expr> = self;
        loop {
            match value {
                Trampoline::Done(done) => return ControlFlow::Continue(done),
                Trampoline::Raise(error) => return ControlFlow::Break(Err(error)),
                Trampoline::Continue(f) => {
                    value = f();
                }
            }
        }
    }
}

impl<T, E, F: From<E>> FromResidual<Result<Infallible, E>> for Trampoline<T, F> {
    fn from_residual(residual: Result<Infallible, E>) -> Self {
        match residual {
            Err(error) => Trampoline::Raise(From::from(error)),
            _ => unreachable!(),
        }
    }
}
