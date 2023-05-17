//! This file specializes the s-expression concrete tree of soft into a more abstracted tree. All
//! the operations should succeded even if the constructs are not well formed so we can emulate
//! runtime errors and make an interpreter.

pub mod tree;

use crate::syntax::Expr;

use tree::{Lifted, LiteralNode, ReferenceType, Term};

use self::tree::IsMacro;

/// Macro to match a pattern and return a result if it matches.
macro_rules! matches {
    ($pattern:pat => $result:expr) => {
        |expr| match expr {
            $pattern => Some($result),
            _ => None,
        }
    };
}

/// Immutable environment that stores important information to the specialization phase like global
/// variables and parameters.
#[derive(Clone, Default)]
pub struct Env<'a> {
    /// Parameters created by lets, lambdas and other constructs.s
    locals: im_rc::HashSet<&'a str>,
}

impl<'a> Env<'a> {
    pub fn add_param(&mut self, param: &'a str) -> Self {
        let mut new = self.clone();
        new.locals.insert(param);
        new
    }

    pub fn get_type(&self, param: &str) -> ReferenceType {
        if self.locals.contains(param) {
            ReferenceType::Local
        } else {
            ReferenceType::Global
        }
    }

    pub fn add_params(&mut self, params: &[&'a str]) -> Self {
        let mut new = self.clone();
        new.locals.extend(params.iter().copied());
        new
    }
}

/// Specializes a list of expressions into a term if it not fits any of the other specializations.
/// This is the default specialization and it always generate an application instead of the specialized
/// versions.
fn fallback_call(env: Env, from: &[Expr]) -> Term {
    if from.is_empty() {
        Term::literal(LiteralNode::Nil)
    } else {
        let mut args = Vec::new();

        for expr in from.iter().skip(1) {
            args.push(from_default(env.clone(), expr));
        }

        Term::application(
            from_default(env.clone(), &from[0]),
            args.into_iter().collect(),
        )
    }
}

/// Specializes a list of expressions into a term if it fits any specialization.
fn from_call(env: Env, from: &[Expr]) -> Term {
    match from {
        [Expr::Id(_, str)] if str == "nil" => Term::literal(LiteralNode::Nil),
        [Expr::Id(_, str), Expr::List(_, args), rest @ ..] if str == "lambda" => {
            let args = args
                .iter()
                .map(matches!(Expr::Id(_, name) => name.clone()))
                .collect::<Option<Vec<String>>>();

            if let Some(args) = args {
                let mut body = Vec::new();

                for expr in rest {
                    body.push(from_default(env.clone(), expr));
                }

                Term::lambda(args, body, Lifted::No)
            } else {
                fallback_call(env, from)
            }
        }
        [Expr::Id(_, str), Expr::Id(_, name), expr] if str == "setm!" => {
            Term::set(name.to_string(), from_default(env, expr), IsMacro::Yes)
        }
        [Expr::Id(_, str), Expr::Id(_, name), expr] if str == "set!" => {
            Term::set(name.to_string(), from_default(env, expr), IsMacro::No)
        }
        [Expr::Id(_, str), x] if str == "quote" => {
            Term::quote(x.clone())
        }
        [Expr::Id(_, str), x, xs] if str == "cons" => {
            Term::cons(from_default(env.clone(), x), from_default(env, xs))
        }
        [Expr::Id(_, str), cond, then, els] if str == "if" => {
            Term::cond(from_default(env.clone(), cond), from_default(env.clone(), then), from_default(env, els))
        }
        _ => fallback_call(env, from),
    }
}

pub fn specialize(from: &Expr) -> Term {
    from_default(Default::default(), from)
}

/// The main function of this module, it converts a s-expression into a specialized term and if it
/// is not specialized it will fallback to the default specialization.
pub fn from_default(env: Env, from: &Expr) -> Term {
    match from {
        Expr::List(_, list) if !list.is_empty() => from_call(env.clone(), list),
        Expr::Id(_, str) => Term::variable(str.to_string(), env.get_type(str)),
        Expr::Symbol(_, name) => Term::atom(name.to_string()),
        Expr::Str(_, str) => Term::literal(LiteralNode::String(str.clone())),
        Expr::Num(_, num) => Term::literal(LiteralNode::Number(*num)),
        Expr::List(_, list) => fallback_call(env, list.as_slice()),
    }
}
