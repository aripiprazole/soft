use im::{HashMap, HashSet};

use crate::specialized::{Lifted, Term};

pub fn closure(env: &mut Vec<String>, subst: HashMap<String, Term>, term: Term) -> Term {
    use Term::*;

    match term {
        GlobalRef(symbol) if env.contains(&symbol) => LocalRef(symbol),
        Quote(quoted) => Quote(box closure(env, subst, *quoted)),
        EnvRef(env_value, index) => EnvRef(box closure(env, subst, *env_value), index),
        Set(name, is_macro, value) => Set(name, is_macro, box closure(env, subst, *value)),
        LocalRef(symbol) | GlobalRef(symbol) if subst.contains_key(&symbol) => {
            subst[&symbol].clone()
        }
        Call(address, args) => Call(
            address,
            args.into_iter()
                .map(|arg| closure(env, subst.clone(), arg))
                .collect(),
        ),
        Closure(env_values, lam) => Closure(
            env_values
                .into_iter()
                .map(|(name, value)| (name, closure(env, subst.clone(), value)))
                .collect(),
            box closure(env, subst, *lam),
        ),
        App(callee, args) => App(
            box closure(env, subst.clone(), *callee),
            args.into_iter()
                .map(|arg| closure(env, subst.clone(), arg))
                .collect(),
        ),
        Lam(Lifted::Yes, args, body) => {
            let mut subst = subst;

            for name in args.iter() {
                env.push(name.clone());
                subst.remove(name);
            }

            let body = box closure(env, subst, *body);

            for _ in args.iter() {
                env.pop();
            }

            Lam(Lifted::Yes, args, body)
        }
        Lam(Lifted::No, mut args, body) => {
            let mut subst = subst;
            for name in args.iter() {
                env.push(name.clone());
                subst.remove(name);
            }

            let fv = free_vars(&body);

            let closure_refs = env
                .iter()
                .cloned()
                .collect::<HashSet<String>>()
                .relative_complement(args.iter().cloned().collect())
                .intersection(fv);

            let new_subst = closure_refs
                .iter()
                .map(|name| {
                    let term = EnvRef(box LocalRef("env".to_string()), name.clone());
                    (name.clone(), term)
                })
                .collect::<HashMap<String, Term>>();

            let is_closure = new_subst.is_empty();

            let body = box closure(env, new_subst, *body);

            for _ in args.iter() {
                env.pop();
            }

            if is_closure {
                Lam(Lifted::Yes, args, body)
            } else {
                let env_values = closure_refs
                    .iter()
                    .map(|name| (name.clone(), LocalRef(name.clone())))
                    .collect();

                args.push("env".to_string());

                Closure(env_values, box Lam(Lifted::Yes, args, body))
            }
        }
        Let(entries, body) => {
            let size = entries.len();
            let mut converted_entries: Vec<(String, Term)> = vec![];
            let mut subst = subst;
            for (name, value) in entries {
                let converted_value = closure(env, subst.clone(), value);
                converted_entries.push((name.clone(), converted_value));
                env.push(name.clone());
                subst.remove(&name);
            }
            let body = box closure(env, subst, *body);

            for _ in 0..size {
                env.pop();
            }

            Let(converted_entries, body)
        }
        Nil | Num(_) | LocalRef(_) | GlobalRef(_) => term,
    }
}

fn free_vars(term: &Term) -> im::HashSet<String> {
    use Term::*;

    match term {
        Closure(_, lam) => free_vars(lam).iter().collect(),
        EnvRef(env_value, _) => free_vars(env_value),
        Set(_, _, value) => free_vars(value),
        Call(_, args) => args.iter().flat_map(free_vars).collect(),
        Quote(quoted) => free_vars(quoted),
        GlobalRef(name) | LocalRef(name) => HashSet::from_iter([name.clone()]),
        App(callee, args) => free_vars(callee).union(args.iter().flat_map(free_vars).collect()),
        Lam(_, names, body) => {
            let mut body_fv = free_vars(body);

            for name in names.iter() {
                body_fv.remove(name);
            }

            body_fv
        }
        Let(entries, body) => {
            let mut fv: HashSet<String> = Default::default();

            for (name, value) in entries.iter() {
                fv.remove(name);
                fv = fv.union(free_vars(value));
            }

            let mut body_fv = free_vars(body);

            for (name, _) in entries.iter() {
                body_fv.remove(name);
            }

            body_fv.union(fv)
        }
        Num(_) | Nil => Default::default(),
    }
}

#[cfg(test)]
mod tests {
    use crate::specialized::Lifted;

    use super::*;

    #[test]
    fn test_convert_closure() {
        let mut env: Vec<String> = vec![];
        let actual = closure(
            &mut env,
            Default::default(),
            Term::Lam(
                Lifted::No,
                vec!["a".to_string()],
                box Term::Lam(
                    Lifted::No,
                    vec!["b".to_string()],
                    box Term::App(
                        box Term::GlobalRef("a".to_string()),
                        vec![Term::GlobalRef("b".to_string())],
                    ),
                ),
            ),
        );

        let expected = Term::Lam(
            Lifted::Yes,
            vec!["a".to_string()],
            box Term::Closure(
                vec![("a".to_string(), Term::LocalRef("a".to_string()))],
                box Term::Lam(
                    Lifted::Yes,
                    vec!["b".to_string(), "env".to_string()],
                    box Term::App(
                        box Term::EnvRef(box Term::LocalRef("env".to_string()), "a".to_string()),
                        vec![Term::LocalRef("b".to_string())],
                    ),
                ),
            ),
        );

        assert_eq!(actual, expected)
    }
}
