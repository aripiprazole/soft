//! Evaluates a bunch of structures like [Value], [Closure] into a weak head normal form. This is
//! the main part of the interpreter.

use crate::environment::Environment;
use crate::error::{Result, RuntimeError};
use crate::value::{CallScope, Closure, Expr, Function, Param, Trampoline, Value};

pub fn match_parameters(params: &[Param], args: &[Value]) -> Result<Vec<(String, Value)>> {
    let mut iter = args.iter().cloned();
    let mut result = vec![];

    for param in params {
        match param {
            Param::Required(name) => {
                let Some(value) = iter.next() else {
                    return Err(RuntimeError::WrongArity(params.len(), args.len()));
                };
                result.push((name.clone(), value.clone()));
            }
            Param::Optional(name, def) => {
                let value = iter.next().unwrap_or(def.clone());
                result.push((name.clone(), value));
            }
            Param::Variadic(name) => {
                result.push((name.clone(), Value::from_iter(iter, None)));
                return Ok(result);
            }
        }
    }

    if iter.next().is_some() {
        Err(RuntimeError::WrongArity(params.len(), args.len()))
    } else {
        Ok(result)
    }
}

impl Closure {
    pub fn apply(&self, args: Vec<Value>, env: &mut Environment) -> Result<Trampoline> {
        let mut location = env.location.clone();
        let mut apply = self;

        let mut ret_head;
        let mut ret;

        let mut args = args;

        env.push(self.frame.name.clone(), false, env.location.clone());

        'first: loop {
            let params = match_parameters(&apply.params, &args)?;

            let frame = env.last_frame();
            frame.location = location.clone();
            *frame = self.frame.clone();

            for (name, value) in params {
                frame.insert(name.clone(), value);
            }

            location = self.expr.span.clone().unwrap_or(location);

            ret = self.expr.clone().expand(env)?;

            'snd: loop {
                match &ret.kind {
                    Expr::Cons(head, tail) => {
                        let (new_args, end) = tail.to_list().unwrap();
                        if let Some(end) = end {
                            return Err(RuntimeError::ExpectedList(end.to_string()));
                        }

                        ret_head = head.clone().run(env)?.clone();

                        match &ret_head.kind {
                            Expr::Function(Function::Closure(closure)) => {
                                args = new_args
                                    .into_iter()
                                    .map(|arg| arg.run(env))
                                    .collect::<Result<Vec<_>>>()?;
                                apply = closure;
                                break;
                            }
                            Expr::Function(Function::Extern(extern_)) => {
                                let scope = CallScope {
                                    args: new_args.to_vec(),
                                    env,
                                    location: ret.span.clone(),
                                };
                                let result = (extern_)(scope)?;
                                match result {
                                    Trampoline::Eval(retu) => {
                                        ret = retu;
                                        continue 'snd;
                                    }
                                    Trampoline::Return(result) => {
                                        env.pop();
                                        return Ok(Trampoline::Return(result));
                                    }
                                }
                            }
                            _ => {
                                return Err(RuntimeError::NotCallable(ret_head));
                            }
                        };
                    }
                    _ => break 'first,
                }
            }
        }

        let res = ret.run(env)?;

        env.pop();

        Ok(Trampoline::Return(res))
    }
}

impl Function {
    pub fn apply(&self, args: Vec<Value>, env: &mut Environment, eval: bool) -> Result<Trampoline> {
        match self {
            Function::Closure(closure) => {
                let loc = env.location.clone();
                let args = if eval {
                    args.into_iter()
                        .map(|arg| arg.run(env))
                        .collect::<Result<Vec<_>>>()?
                } else {
                    args
                };
                env.location = loc;
                closure.apply(args, env)
            }
            Function::Extern(extern_) => {
                let scope = CallScope {
                    args: args.to_vec(),
                    location: env.location.clone().into(),
                    env,
                };
                (extern_)(scope)
            }
        }
    }
}

impl Value {
    fn eval(mut self, env: &mut Environment) -> Result<Trampoline> {
        env.set_location(self.span.clone());

        self = self.expand(env)?;

        match &self.kind {
            Expr::Id(name) => {
                let Some(result) = env.find(name) else {
                    return Err(RuntimeError::UndefinedName(name.clone()));
                };
                Ok(Trampoline::Return(result))
            }
            Expr::Cons(head, tail) => {
                let (args, end) = tail.to_list().unwrap();
                if let Some(end) = end {
                    return Err(RuntimeError::ExpectedList(end.to_string()));
                }
                let head = head.clone().run(env)?;
                match &head.kind {
                    Expr::Function(function) => match function.apply(args, env, true) {
                        Ok(res) => Ok(res),
                        Err(err) => Err(err),
                    },
                    _ => Err(RuntimeError::NotCallable(head)),
                }
            }
            _ => Ok(Trampoline::Return(self)),
        }
    }

    pub fn run(self, env: &mut Environment) -> Result<Value> {
        let mut trampoline = Trampoline::Eval(self);
        loop {
            match trampoline {
                Trampoline::Eval(expr) => trampoline = expr.eval(env)?,
                Trampoline::Return(ret) => return Ok(ret),
            }
        }
    }
}

impl Trampoline {
    pub fn run(self, env: &mut Environment) -> Result<Value> {
        let mut trampoline = self;
        loop {
            match trampoline {
                Trampoline::Eval(expr) => trampoline = expr.eval(env)?,
                Trampoline::Return(ret) => return Ok(ret),
            }
        }
    }
}
