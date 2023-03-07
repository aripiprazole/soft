use im::HashSet;

use crate::specialized::{Lifted, Term};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Closure {
    pub env: HashSet<String>,
    pub subst: HashSet<String>,
}

impl Term {
    pub fn convert(self) -> Self {
        Closure::default().convert(self)
    }
}

impl Closure {
    pub fn new(env: HashSet<String>, subst: HashSet<String>) -> Self {
        Self { env, subst }
    }

    pub fn extend(&mut self) -> Closure {
        self.with_subst(self.subst.clone())
    }

    pub fn with_subst(&self, subst: HashSet<String>) -> Self {
        Closure {
            env: self.env.clone(),
            subst,
        }
    }

    pub fn convert(&mut self, term: Term) -> Term {
        use Term::*;

        match term {
            EnvRef(name) => EnvRef(name),
            Quote(quoted) => Quote(quoted),
            Set(name, is_macro, box value) => Set(name, is_macro, Box::new(self.convert(value))),
            LocalRef(symbol) | GlobalRef(symbol) if self.subst.contains(&symbol) => EnvRef(symbol),
            GlobalRef(symbol) if self.env.contains(&symbol) => LocalRef(symbol),
            If(box cond, box then, box otherwise) => Term::If(
                Box::new(self.convert(cond)),
                Box::new(self.convert(then)),
                Box::new(self.convert(otherwise)),
            ),
            Cons(box head, box tail) => {
                Term::Cons(Box::new(self.convert(head)), Box::new(self.convert(tail)))
            }
            Closure(env_values, box lam) => {
                let env_values = env_values
                    .into_iter()
                    .map(|(name, value)| (name, self.convert(value)))
                    .collect();

                Closure(env_values, Box::new(self.convert(lam)))
            }
            App(box callee, args) => App(
                Box::new(self.convert(callee)),
                args.into_iter().map(|arg| self.convert(arg)).collect(),
            ),
            Lam(Lifted::Yes, args, box term) => {
                let mut closure = self.extend();

                closure.env.extend(args.iter().cloned());
                closure.subst.retain(|name| !args.contains(name));

                let term = Box::new(closure.convert(term));

                Lam(Lifted::Yes, args, term)
            }
            Lam(Lifted::No, mut args, box term) => {
                let mut closure = self.extend();

                closure.env.extend(args.iter().cloned());
                closure.subst.retain(|name| !args.contains(name));

                let fv = free_vars(&term);

                let closure_refs = closure
                    .env
                    .clone()
                    .relative_complement(args.iter().cloned().collect())
                    .intersection(fv);

                let is_closure = closure_refs.is_empty();
                let term = closure.with_subst(closure_refs.clone()).convert(term);

                let result = if is_closure {
                    Lam(Lifted::Yes, args, Box::new(term))
                } else {
                    let env_values = closure_refs
                        .iter()
                        .map(|name| (name.clone(), LocalRef(name.clone())))
                        .collect();

                    args.push("#env".to_string());

                    Closure(env_values, Box::new(Lam(Lifted::Yes, args, Box::new(term))))
                };

                self.convert(result)
            }
            Let(entries, box term) => {
                let mut new_entries: Vec<(String, Term)> = vec![];
                let mut closure = self.extend();

                for (name, value) in entries {
                    let new_value = closure.convert(value);
                    new_entries.push((name.clone(), new_value));
                    closure.env.insert(name.clone());
                    closure.subst.remove(&name);
                }

                let body = Box::new(closure.convert(term));

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
        Set(_, _, value) => free_vars(value),
        GlobalRef(name) | LocalRef(name) => HashSet::from_iter([name.clone()]),
        App(callee, args) => free_vars(callee).union(args.iter().flat_map(free_vars).collect()),
        Cons(head, tail) => free_vars(head).union(free_vars(tail)),
        If(cond, then_branch, else_branch) => free_vars(cond)
            .union(free_vars(then_branch))
            .union(free_vars(else_branch)),
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
        Quote(_) | EnvRef(_) | Num(_) | Nil => Default::default(),
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
            Box::new(Term::Lam(
                Lifted::No,
                vec!["b".to_string()],
                Box::new(Term::App(
                    Box::new(Term::GlobalRef("a".to_string())),
                    vec![Term::GlobalRef("b".to_string())],
                )),
            )),
        ));

        let expected = Term::Lam(
            Lifted::Yes,
            vec!["a".to_string()],
            Box::new(Term::Closure(
                vec![("a".to_string(), Term::LocalRef("a".to_string()))],
                Box::new(Term::Lam(
                    Lifted::Yes,
                    vec!["b".to_string()],
                    Box::new(Term::App(
                        Box::new(Term::EnvRef("a".to_string())),
                        vec![Term::LocalRef("b".to_string())],
                    )),
                )),
            )),
        );

        assert_eq!(actual, expected)
    }
}
