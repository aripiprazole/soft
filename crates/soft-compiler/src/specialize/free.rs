//! This module defines functions for finding free variables inside a term. It's used to do closure
//! convertion.

use im_rc::HashSet;

use crate::location::Spanned;

use super::tree::{Definition, PrimKind, Symbol, TermKind, VariableKind};

pub type MutVars<'a, 'b> = &'b mut HashSet<Symbol<'a>>;
pub type Vars<'a> = HashSet<Symbol<'a>>;

pub trait VarCollector<'a> {
    fn free_vars(&self) -> HashSet<Symbol<'a>> {
        let mut free_vars = HashSet::new();
        self.free_vars_helper(Default::default(), &mut free_vars);
        free_vars
    }

    fn free_vars_helper(&self, bound_vars: Vars<'a>, free_vars: MutVars<'a, '_>);
}

impl<'a, T: VarCollector<'a>> VarCollector<'a> for Spanned<T> {
    fn free_vars_helper(&self, bound_vars: Vars<'a>, free_vars: MutVars<'a, '_>) {
        self.data.free_vars_helper(bound_vars, free_vars);
    }
}

impl<'a, T: VarCollector<'a>> VarCollector<'a> for Box<T> {
    fn free_vars_helper(&self, bound_vars: Vars<'a>, free_vars: MutVars<'a, '_>) {
        self.as_ref().free_vars_helper(bound_vars, free_vars);
    }
}

impl<'a> VarCollector<'a> for PrimKind<'a> {
    fn free_vars_helper(&self, bound_vars: Vars<'a>, free_vars: MutVars<'a, '_>) {
        match self {
            PrimKind::Nil => {}
            PrimKind::TypeOf(t) => t.free_vars_helper(bound_vars, free_vars),
            PrimKind::Vec(vec) => {
                for t in vec {
                    t.free_vars_helper(bound_vars.clone(), free_vars);
                }
            }
            PrimKind::Cons(x, xs) => {
                x.free_vars_helper(bound_vars.clone(), free_vars);
                xs.free_vars_helper(bound_vars, free_vars);
            }
            PrimKind::Head(x) => x.free_vars_helper(bound_vars, free_vars),
            PrimKind::Tail(x) => x.free_vars_helper(bound_vars, free_vars),
            PrimKind::VecIndex(vec, idx) => {
                vec.free_vars_helper(bound_vars.clone(), free_vars);
                idx.free_vars_helper(bound_vars, free_vars);
            }
            PrimKind::VecLength(vec) => vec.free_vars_helper(bound_vars, free_vars),
            PrimKind::VecSet(vec, idx, val) => {
                vec.free_vars_helper(bound_vars.clone(), free_vars);
                idx.free_vars_helper(bound_vars.clone(), free_vars);
                val.free_vars_helper(bound_vars, free_vars);
            }
            PrimKind::Box(bx) => {
                bx.free_vars_helper(bound_vars, free_vars);
            }
            PrimKind::Unbox(bx) => {
                bx.free_vars_helper(bound_vars, free_vars);
            }
            PrimKind::BoxSet(bx, val) => {
                bx.free_vars_helper(bound_vars.clone(), free_vars);
                val.free_vars_helper(bound_vars, free_vars);
            }
            PrimKind::GetEnv(_, _) => {}
            PrimKind::CreateClosure(definition, env) => {
                definition.free_vars_helper(bound_vars.clone(), free_vars);
                for (_, value) in env {
                    value.free_vars_helper(bound_vars.clone(), free_vars);
                }
            }
        }
    }
}

impl<'a> VarCollector<'a> for Definition<'a> {
    fn free_vars_helper(&self, bound_vars: Vars<'a>, free_vars: MutVars<'a, '_>) {
        let mut bound_vars = bound_vars.clone();

        for parameter in &self.parameters {
            bound_vars.insert(parameter.clone());
        }

        self.body.free_vars_helper(bound_vars, free_vars);
    }
}

impl<'a> VarCollector<'a> for TermKind<'a> {
    fn free_vars_helper(&self, bound_vars: Vars<'a>, free_vars: MutVars<'a, '_>) {
        match self {
            TermKind::Atom(_) => {}
            TermKind::Number(_) => {}
            TermKind::String(_) => {}
            TermKind::Bool(_) => {}
            TermKind::Variable(VariableKind::Global(_)) => {}
            TermKind::Variable(VariableKind::Local(_, sym)) => {
                if !bound_vars.contains(sym) {
                    free_vars.insert(sym.clone());
                }
            }
            TermKind::Let(names, next) => {
                let mut bound_vars = bound_vars.clone();

                for (name, value) in names {
                    value.free_vars_helper(bound_vars.clone(), free_vars);
                    bound_vars.insert(name.clone());
                }

                next.free_vars_helper(bound_vars, free_vars);
            }
            TermKind::Set(_, _, value, _) => {
                value.free_vars_helper(bound_vars, free_vars);
            }
            TermKind::Lambda(definition, _) => {
                definition.free_vars_helper(bound_vars, free_vars);
            }
            TermKind::Block(statements) => {
                for sttm in statements {
                    sttm.free_vars_helper(bound_vars.clone(), free_vars);
                }
            }
            TermKind::Quote(_) => {}
            TermKind::If(cond, if_, else_) => {
                cond.free_vars_helper(bound_vars.clone(), free_vars);
                if_.free_vars_helper(bound_vars.clone(), free_vars);
                else_.free_vars_helper(bound_vars, free_vars);
            }
            TermKind::Operation(_, args) => {
                for arg in args {
                    arg.free_vars_helper(bound_vars.clone(), free_vars);
                }
            }
            TermKind::Call(fun, args) => {
                fun.free_vars_helper(bound_vars.clone(), free_vars);
                for arg in args {
                    arg.free_vars_helper(bound_vars.clone(), free_vars);
                }
            }
            TermKind::Prim(prim) => {
                prim.free_vars_helper(bound_vars, free_vars);
            }
        }
    }
}
