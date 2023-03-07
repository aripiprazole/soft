use std::fmt::Display;

use crate::{
    macros::bool_enum,
    runtime::{Value, ValueRef},
    spaced::{Mode, Spaced},
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
    LocalRef(String),
    GlobalRef(String),
    Num(u64),
    Quote(ValueRef),
    If(Box<Term>, Box<Term>, Box<Term>),
    Cons(Box<Term>, Box<Term>),
    Nil,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct SpecializeError {
    pub message: String,
    pub r_source_line: u32,
    pub r_source_column: u32,
    pub r_source_file: String,
}

macro_rules! specialize_error {
    ($message:expr) => {
        Err(SpecializeError {
            message: $message.to_string(),
            r_source_line: line!(),
            r_source_column: column!(),
            r_source_file: file!().to_string(),
        })
    };
}

impl TryFrom<ValueRef> for Term {
    type Error = SpecializeError;

    fn try_from(value: ValueRef) -> Result<Self, Self::Error> {
        value.specialize()
    }
}

impl ValueRef {
    pub fn specialize(&self) -> Result<Term, SpecializeError> {
        use Value::*;

        if self.is_num() {
            return Ok(Term::Num(self.num()));
        }

        match self.to_value() {
            Cons(head, tail) if head.is_num() => {
                let args = cons_to_list(*tail)?;
                Ok(Term::App(box head.specialize()?, args))
            }
            Cons(head, tail) => {
                let args = cons_to_list(*tail)?;

                match head.to_value() {
                    Atom(symbol) if symbol == "quote" => Ok(Term::Quote(*tail)),
                    Atom(symbol) => Ok(specialize_cons(symbol, args)?),
                    _ => Ok(Term::App(box head.specialize()?, args)),
                }
            }
            Atom(symbol) if symbol == "nil" => Ok(Term::Nil),
            Atom(symbol) => Ok(Term::GlobalRef(symbol.clone())),
            Nil => Ok(Term::Nil),
            _ => specialize_error!("Invalid value"),
        }
    }
}

fn specialize_cons(head: &str, tail: Vec<Term>) -> Result<Term, SpecializeError> {
    use Term::*;

    match head {
        "set*" => match tail.as_slice() {
            [GlobalRef(name), value] => Ok(Set(name.clone(), IsMacro::No, box value.clone())),
            _ => specialize_error!("Invalid set*"),
        },
        "cons" => match tail.as_slice() {
            [head, tail] => Ok(Cons(box head.clone(), box tail.clone())),
            _ => specialize_error!("Invalid cons*"),
        },
        "if" => match tail.as_slice() {
            [cond, then_branch, else_branch] => Ok(If(
                box cond.clone(),
                box then_branch.clone(),
                box else_branch.clone(),
            )),
            _ => specialize_error!("Invalid if"),
        },
        "lambda" => match tail.as_slice() {
            [Nil, body] => Ok(Lam(Lifted::No, vec![], box body.clone())),
            [App(box head, args), body] => {
                let arguments = vec![head.clone()]
                    .iter()
                    .chain(args.iter())
                    .map(|arg| match arg {
                        GlobalRef(name) => Ok(name.clone()),
                        _ => specialize_error!("Invalid lambda"),
                    })
                    .collect::<Result<Vec<_>, _>>()?;

                Ok(Lam(Lifted::No, arguments, box body.clone()))
            }
            _ => specialize_error!("Invalid lambda"),
        },
        "let" => match tail.as_slice() {
            [Nil] => specialize_error!("Invalid let"),
            [App(box head, args), body] => {
                let bindings = vec![head.clone()]
                    .iter()
                    .chain(args.iter())
                    .map(|entry| match entry {
                        App(box GlobalRef(name), value) => match value.as_slice() {
                            [value] => Ok((name.clone(), value.clone())),
                            _ => specialize_error!("Invalid let binding"),
                        },
                        _ => specialize_error!("Invalid let binding"),
                    })
                    .collect::<Result<Vec<_>, _>>()?;

                Ok(Let(bindings, box body.clone()))
            }
            _ => specialize_error!("Invalid let"),
        },
        "nil" => match tail.as_slice() {
            [] => Ok(Nil),
            _ => specialize_error!("Invalid nil"),
        },
        _ => Ok(App(box GlobalRef(head.to_owned()), tail)),
    }
}

impl Display for Term {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Term::*;

        match self {
            Lam(lifted, names, body) => {
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
            Let(names, next) => {
                write!(f, "(let")?;

                write!(f, " (")?;

                for (name, value) in names {
                    write!(f, " ({name} {value})")?;
                }

                write!(f, ") ")?;
                write!(f, "{next}")?;
                write!(f, ")")
            }
            App(head, tail) => {
                write!(f, "(~{head}{})", Spaced(Mode::Before, " ", tail))
            }
            Closure(args, body) => {
                let names: Vec<_> = args.iter().map(|x| format!("({} {})", x.0, x.1)).collect();
                write!(
                    f,
                    "(closure* {body} ({}))",
                    Spaced(Mode::Interperse, " ", &names)
                )
            }
            EnvRef(name) => {
                write!(f, "(env-ref {name})")
            }
            Set(name, IsMacro::Yes, value) => {
                write!(f, "(setm* {name} {value})")
            }
            Set(name, IsMacro::No, value) => {
                write!(f, "(set* {name} {value})")
            }
            LocalRef(n) => write!(f, "{n}"),
            GlobalRef(n) => write!(f, "#{n}"),
            Num(n) => write!(f, "{n}"),
            Quote(expr) => write!(f, "'{expr}"),
            Nil => write!(f, "nil"),
            If(cond, then_branch, else_branch) => {
                write!(f, "(if")?;
                write!(f, " {cond}")?;
                write!(f, " {then_branch}")?;
                write!(f, " {else_branch})")
            }
            Cons(head, tail) => {
                write!(f, "(cons*")?;
                write!(f, " {head}")?;
                write!(f, " {tail})")
            }
        }
    }
}

fn cons_to_list(tail: ValueRef) -> Result<Vec<Term>, SpecializeError> {
    let mut list = vec![];
    let mut current = tail;

    while let Value::Cons(head, tail) = current.to_value() {
        list.push(head.specialize()?);
        current = *tail;
    }

    Ok(list)
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

        assert_eq!(value.specialize().unwrap(), term);
    }

    #[test]
    fn test_specialize_cons_if_head_is_not_num_and_is_not_atom() {
        let value = ValueRef::cons(
            ValueRef::nil(),
            ValueRef::cons(ValueRef::new_num(1), ValueRef::nil()),
        );

        let term = Term::App(box Term::Nil, vec![Term::Num(1)]);

        assert_eq!(value.specialize().unwrap(), term);
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

        assert_eq!(value.specialize().unwrap(), term);
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

        assert_eq!(value.specialize().unwrap(), term);
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

        assert_eq!(value.specialize().unwrap(), term);
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

        assert_eq!(value.specialize().unwrap(), term);
    }

    #[test]
    fn test_specialize_cons_matching_quote() {
        let value = ValueRef::cons(
            ValueRef::atom("quote".to_string()),
            ValueRef::atom("foo".to_string()),
        );

        let term = Term::Quote(ValueRef::atom("foo".to_string()));

        assert_eq!(value.specialize().unwrap(), term);
    }
}
