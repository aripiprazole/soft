use crate::environment::Environment;
use crate::error::{Result, RuntimeError};
use crate::value::{CallScope, Closure, ExprKind, Function, Trampoline, Value};

impl Closure {
    pub fn apply(&self, args: Vec<Value>, env: &mut Environment) -> Result<Trampoline> {
        let args = args
            .into_iter()
            .map(|arg| arg.run(env))
            .collect::<Result<Vec<_>>>()?;

        if self.params.len() != args.len() {
            return Err(RuntimeError::WrongArity(self.params.len(), args.len()));
        }

        let frame = env.push_from(self.frame.clone());

        for (name, value) in self.params.iter().zip(args.iter().cloned()) {
            frame.insert(name.clone(), value);
        }

        let body = self.expr.clone().run(env)?;

        env.pop();

        Ok(Trampoline::Return(body))
    }
}

impl Function {
    pub fn apply(&self, args: Vec<Value>, env: &mut Environment) -> Result<Trampoline> {
        match self {
            Function::Closure(closure) => closure.apply(args, env),
            Function::Extern(extern_) => {
                let scope = CallScope {
                    args: args.to_vec(),
                    env,
                };
                (extern_)(scope)
            }
        }
    }
}

impl Value {
    fn eval(self, env: &mut Environment) -> Result<Trampoline> {
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
                    ExprKind::Function(function) => match function.apply(args, env) {
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
                Trampoline::EvalPop(expr) => {
                    trampoline = expr.eval(env)?;
                    env.pop();
                }
            }
        }
    }
}
