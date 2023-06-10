//! This module removes all the closures and replace them by some constructions that are more
//! suitable to compilation.

use std::{cell::RefCell, rc::Rc};

use super::tree::{Function, Let, Number, Symbol, Term, Variable, Visitor};

#[derive(Default, Clone)]
pub struct Ctx {
    pub bound_vars: im_rc::HashMap<String, usize>,
    pub free_vars: Rc<RefCell<im_rc::HashMap<String, usize>>>,
    pub counter: usize,
}

impl Visitor for Ctx {
    fn visit_variable(&mut self, variable: &mut Variable) {
        if let Variable::Local { name, .. } = variable {
            if self.bound_vars.get(name.name()).is_none() {
                let size = self.free_vars.borrow().len();
                *variable = Variable::Env {
                    name: name.clone(),
                    index: *self
                        .free_vars
                        .borrow_mut()
                        .entry(name.name().to_string())
                        .or_insert(size),
                }
            }
        }
    }

    fn visit_let(&mut self, let_: &mut Let) {
        let mut ctx = self.clone();

        for (name, ref mut value) in let_.bindings.iter_mut() {
            ctx.visit_term(value);
            ctx.bound_vars.insert(name.name().to_string(), ctx.counter);
            ctx.counter += 1;
        }

        ctx.visit_term(&mut let_.body);
    }

    fn visit_term(&mut self, expr: &mut Term) {
        match expr {
            Term::Lambda(lambda) => {
                let mut ctx = self.clone();

                ctx.counter = 0;
                ctx.free_vars = Rc::new(RefCell::new(Default::default()));

                ctx.bound_vars = lambda.args.iter().fold(Default::default(), |mut acc, arg| {
                    acc.insert(arg.name().to_string(), ctx.counter);
                    ctx.counter += 1;
                    acc
                });

                let args = lambda.args.clone();
                ctx.visit_term(&mut lambda.body);

                let fv = ctx.free_vars.borrow();
                let mut fv = fv.clone().into_iter().collect::<Vec<_>>();

                fv.sort_by(|a, b| a.1.cmp(&b.1));

                let mut env = vec![];

                for (var, _) in fv {
                    let mut var = Variable::Local {
                        index: *self.bound_vars.get(&var).unwrap_or(&0),
                        name: Symbol::new(var),
                    };
                    self.visit_variable(&mut var);
                    env.push(Term::Variable(var));
                }

                let mut t = Box::new(Term::Number(Number { value: 0 }));
                std::mem::swap(&mut lambda.body, &mut t);

                *expr = Term::CreateClosure(Function {
                    env,
                    params: args,
                    body: t,
                })
            }
            _ => expr.walk(self),
        }
    }
}

impl<'a> Term<'a> {
    pub fn closure_convert(&mut self) {
        let mut ctx = Ctx::default();
        ctx.visit_term(self);
    }
}
