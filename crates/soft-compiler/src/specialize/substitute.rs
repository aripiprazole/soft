//! This module is used to perform substitution on terms.

use im_rc::HashMap;

use crate::location::Spanned;

use super::tree::{Definition, PrimKind, Symbol, TermKind, VariableKind};

pub type Subst<'a> = HashMap<Symbol<'a>, TermKind<'a>>;

pub trait Substitutable<'a> {
    fn substitute(&mut self, subst: Subst<'a>);
}

impl<'a, T: Substitutable<'a>, U> Substitutable<'a> for (U, T) {
    fn substitute(&mut self, subst: Subst<'a>) {
        self.1.substitute(subst);
    }
}

impl<'a, T: Substitutable<'a>> Substitutable<'a> for Spanned<T> {
    fn substitute(&mut self, subst: Subst<'a>) {
        self.data.substitute(subst)
    }
}

impl<'a, T: Substitutable<'a>> Substitutable<'a> for Vec<T> {
    fn substitute(&mut self, subst: Subst<'a>) {
        for t in self {
            t.substitute(subst.clone());
        }
    }
}

impl<'a> Substitutable<'a> for Definition<'a> {
    fn substitute(&mut self, subst: Subst<'a>) {
        let mut subst = subst.clone();

        for param in &self.parameters {
            subst.remove(param);
        }

        if subst.is_empty() {
            return;
        }

        self.body.data.substitute(subst);
    }
}

impl<'a> Substitutable<'a> for PrimKind<'a> {
    fn substitute(&mut self, subst: Subst<'a>) {
        match self {
            PrimKind::TypeOf(t) => t.substitute(subst),
            PrimKind::Vec(vec) => vec.substitute(subst),
            PrimKind::Cons(x, xs) => {
                x.substitute(subst.clone());
                xs.substitute(subst);
            }
            PrimKind::Nil => {}
            PrimKind::Head(x) => x.substitute(subst),
            PrimKind::Tail(x) => x.substitute(subst),
            PrimKind::VecIndex(vec, idx) => {
                vec.substitute(subst.clone());
                idx.substitute(subst);
            }
            PrimKind::VecLength(vec) => vec.substitute(subst),
            PrimKind::VecSet(vec, idx, val) => {
                vec.substitute(subst.clone());
                idx.substitute(subst.clone());
                val.substitute(subst);
            }
            PrimKind::Box(bx) => bx.substitute(subst),
            PrimKind::Unbox(bx) => bx.substitute(subst),
            PrimKind::BoxSet(bx, val) => {
                bx.substitute(subst.clone());
                val.substitute(subst);
            }
            PrimKind::GetEnv(_, _) => {}
            PrimKind::CreateClosure(name, args) => {
                name.substitute(subst.clone());
                for arg in args {
                    arg.1.substitute(subst.clone());
                }
            }
        }
    }
}

impl<'a> Substitutable<'a> for TermKind<'a> {
    fn substitute(&mut self, subst: Subst<'a>) {
        match self {
            TermKind::Atom(_) => {}
            TermKind::Number(_) => {}
            TermKind::String(_) => {}
            TermKind::Bool(_) => {}
            TermKind::Variable(kind) => {
                let name = match kind {
                    VariableKind::Global(name) => name,
                    VariableKind::Local(_, name) => name,
                };

                if let Some(res) = subst.get(name) {
                    *self = res.clone();
                }
            }
            TermKind::Let(bindings, body) => {
                let mut subst = subst.clone();
                for (symbol, value) in bindings {
                    value.substitute(subst.clone());
                    subst.remove(symbol);

                    if subst.is_empty() {
                        return;
                    }
                }
                body.substitute(subst);
            }
            TermKind::Set(_, _, val, _) => {
                val.substitute(subst);
            }
            TermKind::Lambda(params, _) => {
                params.substitute(subst);
            }
            TermKind::Block(stmts) => {
                stmts.substitute(subst);
            }
            TermKind::Quote(_) => {}
            TermKind::If(cond, then, els) => {
                cond.substitute(subst.clone());
                then.substitute(subst.clone());
                els.substitute(subst);
            }
            TermKind::Operation(_, args) => {
                args.substitute(subst);
            }
            TermKind::Call(func, args) => {
                func.substitute(subst.clone());
                args.substitute(subst);
            }
            TermKind::Prim(prim) => {
                prim.substitute(subst);
            }
        }
    }
}
