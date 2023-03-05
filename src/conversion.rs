use im::{HashMap, HashSet};

use crate::specialized::{Lifted, Term};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Closure {
    pub env: HashSet<String>,
    pub subst: HashMap<String, Term>,
}

impl Default for Closure {
    fn default() -> Self {
        Closure::new(HashSet::default(), HashMap::default())
    }
}

impl Closure {
    pub fn new(env: HashSet<String>, subst: HashMap<String, Term>) -> Self {
        Self { env, subst }
    }

    pub fn extend(&mut self) -> Closure {
        self.with_subst(self.subst.clone())
    }

    pub fn with_subst(&self, subst: HashMap<String, Term>) -> Self {
        Closure {
            env: self.env.clone(),
            subst,
        }
    }

    pub fn convert(&mut self, term: Term) -> Term {
        use Term::*;

        match term {
            Quote(quoted) => Quote(box self.convert(*quoted)),
            EnvRef(env_value, index) => EnvRef(box self.convert(*env_value), index),
            Set(name, is_macro, value) => Set(name, is_macro, box self.convert(*value)),
            LocalRef(symbol) | GlobalRef(symbol) if self.subst.contains_key(&symbol) => {
                self.subst[&symbol].clone()
            }
            GlobalRef(symbol) if self.env.contains(&symbol) => LocalRef(symbol),
            Call(address, args) => Call(
                address,
                args.into_iter().map(|arg| self.convert(arg)).collect(),
            ),
            Closure(env_values, lam) => Closure(
                env_values
                    .into_iter()
                    .map(|(name, value)| (name, self.convert(value)))
                    .collect(),
                box self.convert(*lam),
            ),
            App(callee, args) => App(
                box self.convert(*callee),
                args.into_iter().map(|arg| self.convert(arg)).collect(),
            ),
            Lam(Lifted::Yes, args, body) => {
                let mut closure = self.extend();

                for name in args.iter() {
                    closure.env.insert(name.clone());
                    closure.subst.remove(name);
                }

                let body = box closure.convert(*body);

                Lam(Lifted::Yes, args, body)
            }
            Lam(Lifted::No, mut args, body) => {
                let mut closure = self.extend();

                for name in args.iter() {
                    closure.env.insert(name.clone());
                    closure.subst.remove(name);
                }

                let fv = free_vars(&body);

                let closure_refs = closure
                    .env
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

                let body = box closure.with_subst(new_subst).convert(*body);

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
                let mut new_entries: Vec<(String, Term)> = vec![];
                let mut closure = self.extend();

                for (name, value) in entries {
                    let new_value = closure.convert(value);
                    new_entries.push((name.clone(), new_value));
                    closure.env.insert(name.clone());
                    closure.subst.remove(&name);
                }

                let body = box closure.convert(*body);

                Let(new_entries, body)
            }
            Nil | Num(_) | LocalRef(_) | GlobalRef(_) => term,
        }
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
        let actual = Closure::default().convert(Term::Lam(
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
        ));

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
