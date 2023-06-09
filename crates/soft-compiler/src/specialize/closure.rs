//! This module does what is called 'closure conversion' it lifts all of the lambdas with closures
//! to lambdas without closures.

use std::ops::Range;

use im_rc::HashMap;

use crate::location::{Loc, Spanned};

use super::{
    free::{VarCollector, Vars},
    substitute::Substitutable,
    tree::{IsLifted, PrimKind, Symbol, Term, TermKind, VariableKind},
};

#[derive(Default, Clone)]
pub struct BoundVars<'a> {
    vars: im_rc::HashMap<Symbol<'a>, usize>,
    count: usize,
}

impl<'a> BoundVars<'a> {
    pub fn names(&self) -> Vars<'a> {
        self.vars.iter().map(|x| x.0).cloned().collect()
    }
}

fn create_new_ctx_vars<'a>(def: &mut super::tree::Definition<'a>) -> HashMap<Symbol<'a>, usize> {
    def.parameters
        .clone()
        .into_iter()
        .enumerate()
        .map(|(x, y)| (y, x))
        .collect()
}

fn create_get_env_subst<'a>(fv: &im_rc::HashSet<Symbol<'a>>) -> HashMap<Symbol<'a>, TermKind<'a>> {
    fv.clone()
        .into_iter()
        .enumerate()
        .map(|(n, m)| (m.clone(), TermKind::Prim(PrimKind::GetEnv(n, m.clone()))))
        .collect()
}

fn create_local<'a>(x: Symbol<'a>, span: Range<Loc>, ctx: &BoundVars<'a>) -> Spanned<TermKind<'a>> {
    Term::new(
        span,
        TermKind::Variable(VariableKind::Local(*ctx.vars.get(&x).unwrap(), x.clone())),
    )
}

/// This trait is used to perform closure conversion.
pub trait ClosureConvert<'a> {
    fn closure_convert(&mut self) {
        self.closure_convert_help(Default::default())
    }

    fn closure_convert_help(&mut self, bound_vars: BoundVars<'a>);
}

impl<'a, T: ClosureConvert<'a>> ClosureConvert<'a> for Spanned<T> {
    fn closure_convert_help(&mut self, bound_vars: BoundVars<'a>) {
        self.data.closure_convert_help(bound_vars)
    }
}

impl<'a, T: ClosureConvert<'a>> ClosureConvert<'a> for Vec<T> {
    fn closure_convert_help(&mut self, bound_vars: BoundVars<'a>) {
        for elem in self {
            elem.closure_convert_help(bound_vars.clone())
        }
    }
}

impl<'a, T: ClosureConvert<'a>> ClosureConvert<'a> for Box<T> {
    fn closure_convert_help(&mut self, bound_vars: BoundVars<'a>) {
        self.as_mut().closure_convert_help(bound_vars)
    }
}

impl<'a> ClosureConvert<'a> for PrimKind<'a> {
    fn closure_convert_help(&mut self, bound_vars: BoundVars<'a>) {
        match self {
            PrimKind::TypeOf(x) => x.closure_convert_help(bound_vars),
            PrimKind::Vec(x) => x.closure_convert_help(bound_vars),
            PrimKind::Cons(x, xs) => {
                x.closure_convert_help(bound_vars.clone());
                xs.closure_convert_help(bound_vars);
            }
            PrimKind::Head(x) => x.closure_convert_help(bound_vars),
            PrimKind::Tail(x) => x.closure_convert_help(bound_vars),
            PrimKind::Nil => {}
            PrimKind::VecIndex(x, len) => {
                x.closure_convert_help(bound_vars.clone());
                len.closure_convert_help(bound_vars);
            }
            PrimKind::VecLength(x) => {
                x.closure_convert_help(bound_vars);
            }
            PrimKind::VecSet(x, idx, val) => {
                x.closure_convert_help(bound_vars.clone());
                idx.closure_convert_help(bound_vars.clone());
                val.closure_convert_help(bound_vars);
            }
            PrimKind::Box(x) => {
                x.closure_convert_help(bound_vars);
            }
            PrimKind::Unbox(x) => {
                x.closure_convert_help(bound_vars);
            }
            PrimKind::BoxSet(x, val) => {
                x.closure_convert_help(bound_vars.clone());
                val.closure_convert_help(bound_vars);
            }
            PrimKind::GetEnv(_, _) => {}
            PrimKind::CreateClosure(x, env) => {
                x.closure_convert_help(bound_vars.clone());
                for (_, value) in env {
                    value.closure_convert_help(bound_vars.clone());
                }
            }
        }
    }
}

impl<'a> ClosureConvert<'a> for Term<'a> {
    fn closure_convert_help(&mut self, bound_vars: BoundVars<'a>) {
        match &mut self.data {
            TermKind::Atom(_) => {}
            TermKind::Number(_) => {}
            TermKind::String(_) => {}
            TermKind::Bool(_) => {}
            TermKind::Variable(var) => match var {
                VariableKind::Global(_) => {}
                VariableKind::Local(ref mut idx, name) => {
                    if let Some(index) = bound_vars.vars.get(name) {
                        *idx = *index;
                    }
                }
            },
            TermKind::Quote(_) => {}
            TermKind::Set(_, _ast, tree, _) => {
                tree.closure_convert_help(bound_vars);
            }
            TermKind::Block(sttms) => {
                sttms.closure_convert_help(bound_vars);
            }
            TermKind::If(cond, if_, else_) => {
                cond.closure_convert_help(bound_vars.clone());
                if_.closure_convert_help(bound_vars.clone());
                else_.closure_convert_help(bound_vars);
            }
            TermKind::Operation(_, args) => {
                args.closure_convert_help(bound_vars);
            }
            TermKind::Call(fun, args) => {
                fun.closure_convert_help(bound_vars.clone());
                args.closure_convert_help(bound_vars);
            }
            TermKind::Prim(prim) => {
                prim.closure_convert_help(bound_vars);
            }
            TermKind::Let(binds, val) => {
                let mut bound_ctx = bound_vars.clone();

                for (name, value) in binds {
                    value.closure_convert_help(bound_ctx.clone());
                    bound_ctx.vars.insert(name.clone(), bound_ctx.count);
                    bound_ctx.count += 1;
                }

                val.closure_convert_help(bound_ctx);
            }
            TermKind::Lambda(def, mode) if *mode == IsLifted::No => {
                let new_ctx: HashMap<_, _> = create_new_ctx_vars(def);

                let new_ctx = BoundVars {
                    vars: new_ctx,
                    count: def.parameters.len(),
                };

                def.body.closure_convert_help(new_ctx.clone());

                let mut fv = Default::default();
                def.body.free_vars_helper(new_ctx.names(), &mut fv);

                *mode = IsLifted::Yes;

                if !fv.is_empty() {
                    let subst = create_get_env_subst(&fv);

                    def.body.substitute(subst);

                    let mut took = unsafe { std::mem::zeroed() };
                    std::mem::swap(&mut took, self);
                    let span = took.range.clone();
                    let new = TermKind::Prim(PrimKind::CreateClosure(
                        Box::new(took),
                        fv.into_iter()
                            .map(|x| (x.clone(), create_local(x, span.clone(), &bound_vars)))
                            .collect(),
                    ));

                    *self = Spanned::new(span, new);
                }
            }
            TermKind::Lambda(_, _) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::parser::parse;
    use crate::specialize::closure::ClosureConvert;

    #[test]
    pub fn it_works() {
        let code = "(lambda (x z) (lambda (y) (z x)))";
        let parsed = parse(code).unwrap();
        let mut specialized = parsed[0].specialize();
        println!("{}", specialized);
        specialized.closure_convert();
        println!("{}", specialized);
    }
}
