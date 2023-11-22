use std::{
    collections::VecDeque,
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
    pub frames: VecDeque<Frame>,
}

pub enum Trampoline<T> {
    Done(T),
    Raise(String),
    Continue(Box<dyn FnOnce() -> Trampoline<T>>),
}

pub fn eval(expr: Expr, _: Environment) -> Trampoline<Expr> {
    match expr {
        Expr::Fun(_) => todo!(),
        Expr::List(_) => todo!(),
        Expr::Apply(_) => todo!(),
        Expr::Def(_) => todo!(),
        Expr::DefMacro(_) => todo!(),
        Expr::Quote(_) => todo!(),
        Expr::Recur(_) => todo!(),
        Expr::Deref(_) => todo!(),
        Expr::Atomic(_) => todo!(),
        Expr::Set(_) => todo!(),
        Expr::Literal(_) => Trampoline::Done(expr),
    }
}
