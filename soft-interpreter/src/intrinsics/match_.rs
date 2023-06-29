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
    Identifier(Atom),    // `x
    Rest(Atom),          // &rest `x
    List(Vec<Pattern>),  // [..]
    Tuple(Vec<Pattern>), // (..)

    Int(Atom), // (int `x)
    Str(Atom), // (str `x)
}

pub struct Case {
    pub pattern: Pattern,
    pub body: Value,
}

impl Case {
    pub fn run(&self, scope: CallScope<'_>, scrutinee: Value) -> Result<Option<Value>> {
        todo!()
    }
}

impl From<Value> for Pattern {
    fn from(value: Value) -> Self {
        match &value.kind {
            Expr::Int(_) => todo!(),
            Expr::Id(_) => todo!(),
            Expr::Str(_) => todo!(),
            Expr::Cons(_, _) => {
                let (head, None) = value.to_list().unwrap() else {
                    todo!()
                };
                let Some(callee) = head.first() else {
                    return Pattern::Tuple(vec![]);
                };

                match &callee.kind {
                    Expr::Id(name) if name == "int" => todo!(),
                    Expr::Id(name) if name == "str" => todo!(),
                    _ => Pattern::Tuple(head.into_iter().map(|x| x.into()).collect()),
                }
            }
            Expr::Atom(_) => todo!(),
            Expr::Function(_) => todo!(),
            Expr::Err(_, _) => todo!(),
            Expr::Vector(_) => todo!(),
            Expr::HashMap(_) => todo!(),
            Expr::Library(_) => todo!(),
            Expr::External(_, _) => todo!(),
            Expr::Nil => todo!(),
        }
    }
}
