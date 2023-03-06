use crate::{
    runtime::{Value, ValueRef},
    specialized::{Lifted, Term},
};

pub fn interpret(mut env: im::HashMap<String, ValueRef>, term: Term) -> ValueRef {
    match term {
        Term::Lam(Lifted::No, _, _) => panic!("Oh no"),
        Term::Lam(_, name, body) => ValueRef::new(Value::Closure(Default::default(), name, *body)),
        Term::Let(args, body) => {
            for (name, arg) in args {
                env.insert(name, interpret(env.clone(), arg));
            }

            interpret(env, *body)
        }
        Term::App(head, tail) => {
            if let box Term::GlobalRef(x) = &head {
                if x == "+" {
                    return ValueRef::new_num(
                        tail.into_iter()
                            .map(|x| {
                                let res = interpret(env.clone(), x);
                                if !res.is_num() {
                                    panic!("not a number")
                                }
                                res.num()
                            })
                            .sum(),
                    );
                }
            }

            let res = interpret(env.clone(), *head);

            if !res.is_num() {
                match res.to_value() {
                    Value::Closure(closure_env, args, body) => {
                        if args.len() != tail.len() {
                            panic!("expected {} arguments but got {}", args.len(), tail.len())
                        }

                        let evaluated: Vec<_> = tail
                            .into_iter()
                            .map(|x| interpret(env.clone(), x))
                            .collect();

                        for (name, arg) in args.iter().zip(evaluated.into_iter()) {
                            env.insert(name.clone(), arg);
                        }

                        env.insert(
                            "#env".to_string(),
                            ValueRef::new(Value::HashMap(closure_env.clone())),
                        );

                        interpret(env, body.clone())
                    }
                    _ => panic!("cannot apply as function"),
                }
            } else {
                panic!("cannot apply num as function")
            }
        }
        Term::Num(n) => ValueRef::new_num(n),
        Term::Closure(objs, body) => {
            if let Term::Lam(_, name, body) = *body {
                let objs: im::HashMap<_, _> = objs
                    .into_iter()
                    .map(|x| (x.0, interpret(env.clone(), x.1)))
                    .collect();
                ValueRef::new(Value::Closure(objs, name, *body))
            } else {
                panic!("oh no")
            }
        }
        Term::EnvRef(n) => {
            if let Value::HashMap(h) = env.get("#env").unwrap().to_value() {
                *h.get(&n)
                    .unwrap_or_else(|| panic!("oh no... cant get the variable {}", n))
            } else {
                panic!("not an enveiromenteadsd");
            }
        }
        Term::LocalRef(n) => *env
            .get(&n)
            .unwrap_or_else(|| panic!("oh no... cant get the variable {}", n)),
        Term::Quote(_) => todo!(),
        Term::Nil => todo!(),
        Term::Call(_, _) => todo!(),
        Term::GlobalRef(_) => todo!(),
        Term::Set(_, _, _) => todo!(),
    }
}
