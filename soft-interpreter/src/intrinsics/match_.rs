//! Defines a match contruction and pattern match definitions for the language.

use crate::{
    error::{Result, RuntimeError},
    value::{CallScope, Expr, Trampoline, Value},
};

#[derive(Debug)]
pub struct Atom {
    pub name: String,
    pub unquote: bool,
}

#[derive(Debug)]
pub enum Type {
    Id,
    Str,
    Atom,
    Int,
}

#[derive(Debug)]
pub enum Pattern {
    Identifier(Atom),   // `x
    Rest(Atom),         // &rest `x
    List(Vec<Pattern>), // [..]
    Atom(String),
    Is(Type, Atom),
}

pub struct Case {
    pub pattern: Pattern,
    pub body: Vec<Value>,
}

impl Pattern {
    pub fn matches(
        &self,
        bindings: &mut im_rc::HashMap<String, Value>,
        value: Value,
    ) -> Result<bool> {
        match (self, value) {
            (Pattern::Rest(_), _) => Err(RuntimeError::from(
                "rest pattern cannot be used outside of list",
            )),
            (Pattern::Identifier(atom), value) => {
                if atom.unquote {
                    bindings.insert(atom.name.clone(), value);
                    Ok(true)
                } else {
                    Ok(atom.name == "_" || (value.is_id() && value.to_string() == atom.name))
                }
            }
            (Pattern::Atom(name), value) => {
                if let Expr::Atom(atom) = &value.kind {
                    Ok(atom == name)
                } else {
                    Ok(false)
                }
            }
            (Pattern::List(patterns), value) => {
                let Some((head, _ )) = value.to_list() else {
                    return Ok(false)
                };

                for (used, pattern) in patterns.iter().enumerate() {
                    match pattern {
                        Pattern::Rest(atom) => {
                            let rest = Value::from_iter(head[used..].iter().cloned(), None);
                            bindings.insert(atom.name.clone(), rest);
                            return Ok(true);
                        }
                        _ => {
                            if used >= head.len() {
                                return Ok(false);
                            }

                            if !pattern.matches(bindings, head[used].clone())? {
                                return Ok(false);
                            }
                        }
                    }
                }

                Ok(true)
            }
            (Pattern::Is(typ, atom), value) => match typ {
                Type::Id if value.is_id() => {
                    bindings.insert(atom.name.clone(), value);
                    Ok(true)
                }
                Type::Str if value.is_str() => {
                    bindings.insert(atom.name.clone(), value);
                    Ok(true)
                }
                Type::Atom if value.is_atom() => {
                    bindings.insert(atom.name.clone(), value);
                    Ok(true)
                }
                Type::Int if value.is_int() => {
                    bindings.insert(atom.name.clone(), value);
                    Ok(true)
                }
                _ => Ok(false),
            },
        }
    }
}

impl Case {
    pub fn from_values(values: Vec<Value>) -> Option<Self> {
        let (pattern, body) = values.split_first()?;

        if body.is_empty() {
            return None;
        }

        let pattern = Pattern::from(pattern.clone());

        Some(Self {
            pattern,
            body: body.to_vec(),
        })
    }

    pub fn run(&self, scope: &mut CallScope<'_>, scrutinee: Value) -> Result<Option<Trampoline>> {
        let mut bindings = im_rc::HashMap::new();

        if self.pattern.matches(&mut bindings, scrutinee)? {
            scope.env.last_frame().child(bindings);
            let (last, start) = self.body.split_last().unwrap();

            for value in start {
                value.clone().run(scope.env)?;
            }

            Ok(Some(Trampoline::Eval(last.clone())))
        } else {
            Ok(None)
        }
    }
}

impl From<Value> for Pattern {
    fn from(value: Value) -> Self {
        match &value.kind {
            Expr::Id(name) if name.starts_with('&') => {
                let name_without = &name[1..].to_string();

                Pattern::Rest(Atom {
                    name: name_without.to_string(),
                    unquote: false,
                })
            }
            Expr::Id(name) => Pattern::Identifier(Atom {
                name: name.to_string(),
                unquote: false,
            }),
            Expr::Cons(_, _) => {
                let (head, None) = value.to_list().unwrap() else {
                    todo!()
                };

                let Some((callee, tail)) = head.split_first() else {
                    return Pattern::List(vec![]);
                };

                match &callee.kind {
                    Expr::Id(name) if name == "id" => {
                        let Some((head, [])) = tail.split_first() else {
                            todo!("id requires at one argument")
                        };

                        match &head.clone().kind {
                            Expr::Id(name) => Pattern::Is(
                                Type::Id,
                                Atom {
                                    name: name.to_string(),
                                    unquote: false,
                                },
                            ),
                            _ => todo!("id requires an identifier"),
                        }
                    }
                    Expr::Id(name) if name == "str" => {
                        let Some((head, [])) = tail.split_first() else {
                            todo!("str requires at one argument")
                        };

                        match &head.clone().kind {
                            Expr::Id(name) => Pattern::Is(
                                Type::Str,
                                Atom {
                                    name: name.to_string(),
                                    unquote: false,
                                },
                            ),
                            _ => todo!("str requires an identifier"),
                        }
                    }
                    Expr::Id(name) if name == "atom" => {
                        let Some((head, [])) = tail.split_first() else {
                            todo!("atom requires at one argument")
                        };

                        match &head.clone().kind {
                            Expr::Id(name) => Pattern::Is(
                                Type::Atom,
                                Atom {
                                    name: name.to_string(),
                                    unquote: false,
                                },
                            ),
                            _ => todo!("atom requires an identifier"),
                        }
                    }
                    Expr::Id(name) if name == "int" => {
                        let Some((head, [])) = tail.split_first() else {
                            todo!("int requires at one argument")
                        };

                        match &head.clone().kind {
                            Expr::Id(name) => Pattern::Is(
                                Type::Int,
                                Atom {
                                    name: name.to_string(),
                                    unquote: false,
                                },
                            ),
                            _ => todo!("int requires an identifier"),
                        }
                    }
                    Expr::Id(name) if name == "unquote" => {
                        let Some((head, [])) = tail.split_first() else {
                            todo!("unquote requires one argument")
                        };

                        match &head.clone().kind {
                            Expr::Id(name) => Pattern::Identifier(Atom {
                                name: name.to_string(),
                                unquote: true,
                            }),
                            _ => todo!("unquote requires an identifier"),
                        }
                    }
                    Expr::Id(name) if name == "list" => {
                        Pattern::List(tail.iter().cloned().map(Pattern::from).collect())
                    }
                    _ => Pattern::List(head.into_iter().map(|x| x.into()).collect()),
                }
            }
            Expr::Atom(atom) => Pattern::Atom(atom.clone()),
            _ => todo!("invalid pattern"),
        }
    }
}

/// match : (scrutinee: a) -> (...cases: (pattern, b)) -> b
///
/// ```lisp
/// (match [1 2 3]
///     ([x y z] (+ x y z)
///     (_       (println "otherwise")))
///
/// (match [1 2 3]
///     ([(int `x) (int `y) (int `z)] (+ x y z)
///     (_       (println "otherwise")))
///
/// (match [1 2 "String"]
///     ([(int `x) (int `y) (str `z)]
///         (block (+ x y)
///                (println "matched"))
///     (_       (println "otherwise")))
///
/// (match '(block 1 2 3)
///     ((block `x)            1)
///     ((block &rest `x)      2)
///     ((lambda (&rest x) `y) 3)
///     (_                     (println "otherwise")))
/// ```
pub fn match_(mut scope: CallScope<'_>) -> Result<Trampoline> {
    scope.assert_at_least(1)?;

    let scrutinee = scope.at(0).run(scope.env)?;

    let cases = scope
        .args
        .iter()
        .skip(1)
        .cloned()
        .map(|x| x.assert_list())
        .collect::<Result<Vec<_>>>()?;

    for case in cases {
        if case.len() < 2 {
            return Err(RuntimeError::from("invalid case"));
        }

        let Some(case) = Case::from_values(case) else {
            return Err(RuntimeError::from("invalid case"));
        };

        if let Some(trampoline) = case.run(&mut scope, scrutinee.clone())? {
            return Ok(trampoline);
        }
    }

    Err(RuntimeError::from(format!(
        "match: no match for '{scrutinee}'"
    )))
}
