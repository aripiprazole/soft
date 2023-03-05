use im::{HashMap, HashSet};

use crate::specialized::{Lifted, Term};

pub fn convert_closure(env: &mut Vec<String>, subst: HashMap<String, Term>, closure: Term) -> Term {
    match closure {
        Term::LocalRef(symbol) if subst.contains_key(&symbol) => subst[&symbol].clone(),
        Term::GlobalRef(symbol) if subst.contains_key(&symbol) => subst[&symbol].clone(),
        Term::GlobalRef(symbol) if env.contains(&symbol) => Term::LocalRef(symbol),
        Term::Quote(quoted) => Term::Quote(box convert_closure(env, subst, *quoted)),
        Term::EnvRef(env_value, index) => {
            Term::EnvRef(box convert_closure(env, subst, *env_value), index)
        }
        Term::Set(name, is_macro, value) => {
            Term::Set(name, is_macro, box convert_closure(env, subst, *value))
        }
        Term::Call(address, args) => Term::Call(
            address,
            args.into_iter()
                .map(|arg| convert_closure(env, subst.clone(), arg))
                .collect(),
        ),
        Term::Closure(env_values, lam) => Term::Closure(
            env_values
                .into_iter()
                .map(|(name, value)| (name, convert_closure(env, subst.clone(), value)))
                .collect(),
            box convert_closure(env, subst.clone(), *lam),
        ),
        Term::App(callee, args) => Term::App(
            box convert_closure(env, subst.clone(), *callee),
            args.into_iter()
                .map(|arg| convert_closure(env, subst.clone(), arg))
                .collect(),
        ),
        Term::Lam(Lifted::No, mut args, body) => {
            let mut subst = subst;
            for name in args.iter() {
                env.push(name.clone());
                subst.remove(name);
            }

            let fv = free_vars(&mut vec![], &body);

            let closure_refs = env
                .iter()
                .cloned()
                .collect::<HashSet<String>>()
                .relative_complement(args.iter().cloned().collect())
                .intersection(fv);

            let new_subst = closure_refs
                .iter()
                .map(|name| {
                    let term = Term::EnvRef(box Term::LocalRef("env".to_string()), name.clone());
                    (name.clone(), term)
                })
                .collect::<HashMap<String, Term>>();

            let is_closure = new_subst.is_empty();

            let body = box convert_closure(env, new_subst, *body);

            for _ in args.iter() {
                env.pop();
            }

            if is_closure {
                Term::Lam(Lifted::Yes, args, body)
            } else {
                let env_values = closure_refs
                    .iter()
                    .map(|name| (name.clone(), Term::LocalRef(name.clone())))
                    .collect();

                args.push("env".to_string());

                Term::Closure(env_values, box Term::Lam(Lifted::Yes, args, body))
            }
        }
        Term::Let(entries, body) => {
            let size = entries.len();
            let mut converted_entries: Vec<(String, Term)> = vec![];
            let mut subst = subst;
            for (name, value) in entries {
                let converted_value = convert_closure(env, subst.clone(), value);
                converted_entries.push((name.clone(), converted_value));
                env.push(name.clone());
                subst.remove(&name);
            }
            let body = box convert_closure(env, subst, *body);

            for _ in 0..size {
                env.pop();
            }

            Term::Let(converted_entries, body)
        }
        _ => closure,
    }
}

fn free_vars(env: &mut Vec<String>, term: &Term) -> im::HashSet<String> {
    match term {
        Term::Closure(_, lam) => free_vars(env, lam).iter().collect(),
        Term::EnvRef(env_value, _) => free_vars(env, env_value),
        Term::Set(_, _, value) => free_vars(env, value),
        Term::Call(_, args) => args.iter().flat_map(|arg| free_vars(env, arg)).collect(),
        Term::Quote(quoted) => free_vars(env, quoted),
        Term::GlobalRef(name) => HashSet::from_iter([name.clone()]),
        Term::App(callee, args) => args
            .iter()
            .flat_map(|arg| free_vars(env, arg))
            .collect::<HashSet<_>>()
            .union(free_vars(env, callee)),
        Term::Lam(_, names, body) => {
            for name in names.iter() {
                env.push(name.clone());
            }

            let mut body_fv = free_vars(env, body);

            for name in names.iter() {
                env.pop();
                body_fv.remove(name);
            }

            body_fv
        }
        Term::Let(entries, body) => {
            let mut fv: HashSet<String> = Default::default();
            for (name, value) in entries.iter() {
                fv = fv.union(free_vars(env, value));
                env.push(name.clone());
            }

            let mut body_fv = fv.union(free_vars(env, body));

            for (name, _) in entries.iter() {
                env.pop();
                body_fv.remove(name);
            }

            body_fv
        }
        _ => Default::default(),
    }
}

#[cfg(test)]
mod tests {
    use crate::specialized::Lifted;

    use super::*;

    #[test]
    fn test_convert_closure() {
        let mut env: Vec<String> = vec![];
        let actual = convert_closure(
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
