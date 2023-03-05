use crate::specialized::Term;

pub fn convert_closure(env: &mut Vec<String>, closure: Term) -> Term {
    match closure {
        Term::Lam(_, _, _) => todo!(),
        // Term::GlobalRef(symbol) if env.contains(&symbol) => Term::LocalRef(symbol),
        Term::Quote(quoted) => Term::Quote(box convert_closure(env, *quoted)),
        Term::EnvRef(env_value, index) => Term::EnvRef(box convert_closure(env, *env_value), index),
        Term::Set(name, is_macro, value) => {
            env.push(name.clone());
            Term::Set(name, is_macro, box convert_closure(env, *value))
        }
        Term::Call(address, args) => Term::Call(
            address,
            args.into_iter()
                .map(|arg| convert_closure(env, arg))
                .collect(),
        ),
        Term::Closure(env_values, lam) => Term::Closure(
            env_values
                .into_iter()
                .map(|value| convert_closure(env, value))
                .collect(),
            box convert_closure(env, *lam),
        ),
        Term::App(callee, args) => Term::App(
            box convert_closure(env, *callee),
            args.into_iter()
                .map(|arg| convert_closure(env, arg))
                .collect(),
        ),
        Term::Let(entries, body) => {
            let mut converted_entries: Vec<(String, Term)> = vec![];
            let size = entries.len();
            for (name, value) in entries {
                let converted_value = convert_closure(env, value);
                converted_entries.push((name.clone(), converted_value));
                env.push(name.clone());
            }
            let body = box convert_closure(env, *body);

            for _ in 0..size {
                env.pop();
            }

            Term::Let(converted_entries, body)
        }
        _ => closure,
    };
    todo!()
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
                vec![Term::LocalRef(0)],
                box Term::Lam(
                    Lifted::Yes,
                    vec!["env".to_string(), "b".to_string()],
                    box Term::App(
                        box Term::EnvRef(box Term::LocalRef(0), 0),
                        vec![Term::LocalRef(1)],
                    ),
                ),
            ),
        );

        assert_eq!(actual, expected)
    }
}
