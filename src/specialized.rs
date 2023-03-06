use std::fmt::Display;

use crate::{
    runtime::{Value, ValueRef},
    util::{bool_enum, Mode, Spaced},
};

bool_enum!(IsMacro);
bool_enum!(Lifted);

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Term {
    Lam(Lifted, Vec<String>, Box<Term>),
    Let(Vec<(String, Term)>, Box<Term>),
    App(Box<Term>, Vec<Term>),
    Closure(Vec<(String, Term)>, Box<Term>),
    EnvRef(String),
    Set(String, IsMacro, Box<Term>),
    Call(u64, Vec<Term>),
    LocalRef(String),
    GlobalRef(String),
    Num(u64),
    Quote(Box<Term>),
    Nil,
}

impl Display for Term {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Term::Lam(lifted, names, body) => {
                write!(f, "(lambda")?;

                if let Lifted::Yes = lifted {
                    write!(f, "*")?;
                }

                write!(f, " (")?;
                write!(f, "{}", Spaced(Mode::Interperse, " ", names))?;
                write!(f, ") ")?;
                write!(f, "{body}")?;
                write!(f, ")")
            }
            Term::Let(names, next) => {
                write!(f, "(let")?;

                write!(f, " (")?;

                for (name, value) in names {
                    write!(f, " ({name} {value})")?;
                }

                write!(f, ") ")?;
                write!(f, "{next}")?;
                write!(f, ")")
            }
            Term::App(head, tail) => {
                write!(f, "(~{head}{})", Spaced(Mode::Before, " ", tail))
            }
            Term::Closure(args, body) => {
                let names: Vec<_> = args.iter().map(|x| format!("({} {})", x.0, x.1)).collect();
                write!(
                    f,
                    "(mk-closure {body} ({}))",
                    Spaced(Mode::Interperse, " ", &names)
                )
            }
            Term::EnvRef(name) => {
                write!(f, "(env-ref {name})")
            }
            Term::Set(name, IsMacro::Yes, value) => {
                write!(f, "(setm* {name} {value})")
            }
            Term::Set(name, IsMacro::No, value) => {
                write!(f, "(set* {name} {value})")
            }
            Term::Call(head, tail) => {
                write!(f, "({head}{})", Spaced(Mode::Before, " ", tail))
            }
            Term::LocalRef(n) => write!(f, "{n}"),
            Term::GlobalRef(n) => write!(f, "#{n}"),
            Term::Num(n) => write!(f, "{n}"),
            Term::Quote(expr) => write!(f, "'{expr}"),
            Term::Nil => write!(f, "nil"),
        }
    }
}

impl Term {
    pub fn specialize(value: ValueRef) -> Term {
        if value.is_num() {
            return Term::Num(value.num());
        }

        match value.to_value() {
            Value::Cons(head, tail) if head.is_num() => {
                let args = cons_to_list(*tail);
                Term::App(box Term::specialize(*head), args)
            }
            Value::Cons(head, tail) => {
                let args = cons_to_list(*tail);
                match head.to_value() {
                    Value::Atom(symbol) => Term::specialize_cons(symbol, args),
                    _ => Term::App(box Term::specialize(*head), args),
                }
            }
            Value::Atom(symbol) => Term::GlobalRef(symbol.clone()),
            Value::Nil => Term::Nil,
        }
    }

    fn specialize_cons(head: &str, tail: Vec<Term>) -> Term {
        match head {
            "set*" => match tail.as_slice() {
                [Term::GlobalRef(name), value] => {
                    Term::Set(name.clone(), IsMacro::No, box value.clone())
                }
                _ => todo!(),
            },
            "lambda" => match tail.as_slice() {
                [Term::Nil, body] => Term::Lam(Lifted::No, vec![], box body.clone()),
                [Term::App(head, args), body] => {
                    let arguments = vec![*head.clone()]
                        .iter()
                        .chain(args.iter())
                        .map(|arg| match arg {
                            Term::GlobalRef(name) => name.clone(),
                            _ => todo!(),
                        })
                        .collect();

                    Term::Lam(Lifted::No, arguments, box body.clone())
                }
                _ => todo!(),
            },
            "let" => match tail.as_slice() {
                [Term::Nil] => todo!(),
                [Term::App(head, args), body] => {
                    let bindings = vec![*head.clone()]
                        .iter()
                        .chain(args.iter())
                        .map(|entry| match entry {
                            Term::App(box Term::GlobalRef(name), value) => match value.as_slice() {
                                [value] => (name.clone(), value.clone()),
                                _ => todo!(),
                            },
                            _ => todo!(),
                        })
                        .collect();

                    Term::Let(bindings, box body.clone())
                }
                _ => todo!(),
            },
            "quote" => match tail.as_slice() {
                [value] => Term::Quote(box value.clone()),
                _ => todo!(),
            },
            _ => Term::App(box Term::GlobalRef(head.to_owned()), tail),
        }
    }
}

fn cons_to_list(tail: ValueRef) -> Vec<Term> {
    let mut list = vec![];
    let mut current = tail;

    while let Value::Cons(head, tail) = current.to_value() {
        list.push(Term::specialize(*head));
        current = *tail;
    }

    list
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::ValueRef;

    #[test]
    fn test_specialize_cons_if_head_is_num() {
        let value = ValueRef::cons(
            ValueRef::new_num(1),
            ValueRef::cons(ValueRef::new_num(2), ValueRef::nil()),
        );

        let term = Term::App(box Term::Num(1), vec![Term::Num(2)]);

        assert_eq!(Term::specialize(value), term);
    }

    #[test]
    fn test_specialize_cons_if_head_is_not_num_and_is_not_atom() {
        let value = ValueRef::cons(
            ValueRef::nil(),
            ValueRef::cons(ValueRef::new_num(1), ValueRef::nil()),
        );

        let term = Term::App(box Term::Nil, vec![Term::Num(1)]);

        assert_eq!(Term::specialize(value), term);
    }

    #[test]
    fn test_specialize_cons_matching_set() {
        let value = ValueRef::cons(
            ValueRef::atom("set*".to_string()),
            ValueRef::cons(
                ValueRef::atom("n".to_string()),
                ValueRef::cons(ValueRef::new_num(1), ValueRef::nil()),
            ),
        );

        let term = Term::Set("n".to_string(), IsMacro::No, box Term::Num(1));

        assert_eq!(Term::specialize(value), term);
    }

    #[test]
    fn test_specialize_cons_matching_lambda_without_params() {
        let value = ValueRef::cons(
            ValueRef::atom("lambda".to_string()),
            ValueRef::cons(
                ValueRef::nil(),
                ValueRef::cons(ValueRef::new_num(1), ValueRef::nil()),
            ),
        );

        let term = Term::Lam(Lifted::No, vec![], box Term::Num(1));

        assert_eq!(Term::specialize(value), term);
    }

    #[test]
    fn test_specialize_cons_matching_lambda_with_params() {
        let value = ValueRef::cons(
            ValueRef::atom("lambda".to_string()),
            ValueRef::cons(
                ValueRef::cons(
                    ValueRef::atom("n".to_string()),
                    ValueRef::cons(ValueRef::atom("m".to_string()), ValueRef::nil()),
                ),
                ValueRef::cons(ValueRef::new_num(1), ValueRef::nil()),
            ),
        );

        let term = Term::Lam(
            Lifted::No,
            vec!["n".to_string(), "m".to_string()],
            box Term::Num(1),
        );

        assert_eq!(Term::specialize(value), term);
    }

    #[test]
    fn test_specialize_cons_matching_let() {
        let value = ValueRef::cons(
            ValueRef::atom("let".to_string()),
            ValueRef::cons(
                ValueRef::cons(
                    ValueRef::cons(
                        ValueRef::atom("n".to_string()),
                        ValueRef::cons(ValueRef::new_num(1), ValueRef::nil()),
                    ),
                    ValueRef::nil(),
                ),
                ValueRef::cons(ValueRef::new_num(1), ValueRef::nil()),
            ),
        );

        let term = Term::Let(vec![("n".to_string(), Term::Num(1))], box Term::Num(1));

        assert_eq!(Term::specialize(value), term);
    }

    #[test]
    fn test_specialize_cons_matching_quote() {
        let value = ValueRef::cons(
            ValueRef::atom("quote".to_string()),
            ValueRef::cons(ValueRef::atom("foo".to_string()), ValueRef::nil()),
        );

        let term = Term::Quote(box Term::GlobalRef("foo".to_string()));

        assert_eq!(Term::specialize(value), term);
    }
}
