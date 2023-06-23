//! Evaluates a bunch of structures like [Value], [Closure] into a weak head normal form. This is
//! the main part of the interpreter.

use crate::environment::Environment;
use crate::error::{Result, RuntimeError};
use crate::value::{CallScope, Closure, ExprKind, Function, Trampoline, Value};

impl Closure {
    pub fn apply(&self, args: Vec<Value>, env: &mut Environment) -> Result<Trampoline> {
        let mut location = env.location.clone();
        let mut apply = self;

        let mut ret_head;
        let mut ret;

        let mut args = args;

        env.push(self.frame.name.clone(), false, env.location.clone());

        loop {
            if apply.params.len() != args.len() {
                return Err(RuntimeError::WrongArity(self.params.len(), args.len()));
            }

            let frame = env.last_frame();

            *frame = self.frame.clone();
            frame.location = location.clone();

            for (name, value) in self.params.iter().zip(args.iter().cloned()) {
                frame.insert(name.clone(), value);
            }

            location = self.expr.span.clone().unwrap_or(location);

            ret = self.expr.clone().expand(env)?;

            match &ret.kind {
                ExprKind::Cons(head, tail) => {
                    let (new_args, end) = tail.to_list().unwrap();

                    if let Some(end) = end {
                        return Err(RuntimeError::ExpectedList(end.to_string()));
                    }

                    ret_head = head.clone().run(env)?.clone();

                    match &ret_head.kind {
                        ExprKind::Function(Function::Closure(closure)) => {
                            args = new_args
                                .into_iter()
                                .map(|arg| arg.run(env))
                                .collect::<Result<Vec<_>>>()?;
                            apply = closure;
                        }
                        ExprKind::Function(Function::Extern(extern_)) => {
                            let scope = CallScope {
                                args: new_args.to_vec(),
                                env,
                                location: ret.span.clone(),
                            };
                            let result = (extern_)(scope)?.run(env)?;
                            env.pop();
                            return Ok(Trampoline::Return(result));
                        }
                        _ => {
                            return Err(RuntimeError::NotCallable(ret_head));
                        }
                    };
                }
                _ => break,
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
            ExprKind::Id(name) => {
                let Some(result) = env.find(name) else {
                    return Err(RuntimeError::UndefinedName(name.clone()));
                };
                Ok(Trampoline::Return(result))
            }
            ExprKind::Cons(head, tail) => {
                let (args, end) = tail.to_list().unwrap();
                if let Some(end) = end {
                    return Err(RuntimeError::ExpectedList(end.to_string()));
                }
                let head = head.clone().run(env)?;
                match &head.kind {
                    ExprKind::Function(function) => match function.apply(args, env, true) {
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
