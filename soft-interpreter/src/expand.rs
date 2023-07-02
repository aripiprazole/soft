//! Macro expansion module. It performs the expansion of macros before the execution of functions.

use crate::{
    environment::Environment,
    error::RuntimeError::{self, ExpectedList},
    value::{
        Expr::{Cons, Function, Id},
        Value,
    },
};

use crate::error::Result;

impl Value {
    pub fn expand(self, env: &mut Environment) -> Result<Value> {
        let mut expr = self;
        loop {
            match &expr.kind {
                Id(name) => match env.get_def(name) {
                    Ok(def) if def.is_macro => expr = def.value.clone(),
                    _ => return Ok(expr),
                },
                Cons(head, tail) => match &head.kind {
                    Id(name) if env.get_def(name).map(|x| x.is_macro).unwrap_or(false) => {
                        let Some((args, end)) = tail.to_list() else {
                            return Err(ExpectedList(expr.to_string()));
                        };

                        if let Some(end) = end {
                            return Err(ExpectedList(end.to_string()));
                        }

                        let head = env.get_def(name).unwrap();

                        match &head.value.kind {
                            Function(function) => match function.apply(args, env, false) {
                                Ok(res) => {
                                    expr = res.run(env)?;
                                }
                                Err(err) => {
                                    return Err(err);
                                }
                            },
                            _ => {
                                return Err(RuntimeError::from(format!(
                                    "not callable: {}",
                                    head.value.clone()
                                )))
                            }
                        }
                    }
                    _ => return Ok(expr),
                },
                _ => return Ok(expr),
            }
        }
    }
}
