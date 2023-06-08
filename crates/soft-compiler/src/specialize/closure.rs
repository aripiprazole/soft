//! This module does what is called 'closure conversion' it lifts all of the lambdas with closures
//! to lambdas without closures.

use im_rc::HashMap;

use crate::location::Spanned;

use super::{
    free::VarCollector,
    substitute::Substitutable,
    tree::{IsLifted, PrimKind, Symbol, Term, TermKind, VariableKind},
};

pub type BoundVars<'a> = im_rc::HashMap<Symbol<'a>, usize>;

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
            PrimKind::GetEnv(_) => {}
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
                    *idx = *bound_vars.get(name).unwrap();
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
                    bound_ctx.insert(name.clone(), bound_ctx.len());
                }

                val.closure_convert_help(bound_ctx);
            }
            TermKind::Lambda(def, mode) if *mode == IsLifted::No => {
                let bound_vars: HashMap<_, _> = def
                    .parameters
                    .clone()
                    .into_iter()
                    .enumerate()
                    .map(|(x, y)| (y, x))
                    .collect();

                def.body.closure_convert_help(bound_vars.clone());

                let mut fv = Default::default();
                def.body
                    .free_vars_helper(bound_vars.iter().map(|x| x.0).cloned().collect(), &mut fv);

                *mode = IsLifted::Yes;

                if !fv.is_empty() {
                    let subst = fv
                        .clone()
                        .into_iter()
                        .map(|name| (name.clone(), TermKind::Prim(PrimKind::GetEnv(name.clone()))))
                        .collect();

                    def.body.substitute(subst);

                    let mut took = unsafe { std::mem::zeroed() };
                    std::mem::swap(&mut took, self);
                    let span = took.loc.clone();
                    let new = TermKind::Prim(PrimKind::CreateClosure(
                        Box::new(took),
                        fv.into_iter()
                            .map(|x| {
                                (
                                    x.clone(),
                                    Term::new(
                                        span.clone(),
                                        TermKind::Variable(VariableKind::Local(0, x.clone())),
                                    ),
                                )
                            })
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
    use crate::specialize::closure::ClosureConvert;
    use crate::{parser::parse, specialize::specialize};

    #[test]
    pub fn it_works() {
        let code = "(lambda (z) (lambda (x) (lambda (y) (x z))))";
        let parsed = parse(code).unwrap();
        let mut specialized = specialize(parsed[0].clone());
        println!("{}", specialized);
        specialized.closure_convert();
        println!("{}", specialized);
    }
}
