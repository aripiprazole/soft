use std::collections::HashMap;

use crate::{
    error::Result,
    value::{CallScope, Expr, Trampoline, Value},
};

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
pub fn match_(scope: CallScope<'_>) -> Result<Trampoline> {
    scope.assert_at_least(1)?;

    let scrutinee = scope.at(0).run(scope.env)?;
    let cases = scope
        .args
        .into_iter()
        .skip(1)
        .map(|x| x.run(scope.env))
        .collect::<Result<Vec<_>>>()?;

    todo!()
}

pub struct Atom {
    pub name: String,
    pub unquote: bool,
}

pub enum Pattern {
    Identifier(Atom),   // `x
    Rest(Atom),         // &rest `x
    List(Vec<Pattern>), // [..]

    Int(Atom), // (int `x)
    Str(Atom), // (str `x)
    Id(Atom),
}

pub struct Case {
    pub pattern: Pattern,
    pub body: Vec<Value>,
}

impl Pattern {
    pub fn matches(&self, bindings: &mut HashMap<String, Value>, value: Value) {
        match (self, value) {
            (Pattern::Identifier(atom), value) => {
                bindings.insert(atom.name.clone(), value);
            }
            _ => {}
        }
    }
}

impl Case {
    pub fn from_value(value: Value) -> Option<Self> {
        let Some((values, None)) = value.to_list() else {
            return None;
        };

        let (pattern, body) = values.split_first()?;
        let pattern = Pattern::from(pattern.clone());

        Some(Self {
            pattern,
            body: body.to_vec(),
        })
    }

    pub fn run(&self, scope: CallScope<'_>, scrutinee: Value) -> Result<Option<Value>> {
        todo!()
    }
}

impl From<Value> for Pattern {
    fn from(value: Value) -> Self {
        match &value.kind {
            Expr::Id(name) => Pattern::Identifier(Atom {
                name: name.to_string(),
                unquote: false,
            }),
            Expr::Cons(_, _) => {
                let (head, None) = value.to_list().unwrap() else {
                    todo!()
                };

                let Some(callee) = head.first() else {
                    return Pattern::List(vec![]);
                };

                match &callee.kind {
                    Expr::Id(name) if name == "int" => todo!(),
                    Expr::Id(name) if name == "str" => todo!(),
                    Expr::Id(name) if name == "unquote" => todo!(),
                    Expr::Id(name) if name == "list" => todo!(),
                    _ => Pattern::List(head.into_iter().map(|x| x.into()).collect()),
                }
            }
            _ => todo!(),
        }
    }
}
