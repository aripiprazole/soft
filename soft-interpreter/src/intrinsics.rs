//! This module defines a bunch of functions that are used by the runtime. All the functions are
//! used by the built-in data types.

use crate::*;

/// (defn call* (name))
pub fn call(scope: CallScope<'_>) -> Result<Value> {
    scope.assert_arity(1)?;

    let arg = scope.at(0);
    let name = arg.assert_identifier()?;

    if let Some(res) = scope.env.get(&name) {
        if scope.env.mode == Mode::Macro && scope.env.frames[0].is_macro.contains(&name) {
            scope.env.expanded = true;
            Ok(res)
        } else if scope.env.mode == Mode::Eval {
            Ok(res)
        } else {
            Ok(arg)
        }
    } else if scope.env.mode == Mode::Eval {
        Err(RuntimeError::UndefinedName(name))
    } else {
        Ok(arg)
    }
}

/// (defn set* (name value))
pub fn set(scope: CallScope<'_>) -> Result<Value> {
    scope.assert_arity(2)?;

    let name = scope.at(0).assert_identifier()?;
    let value = scope.at(1).eval(scope.env)?;

    scope.env.frames[0].variables.insert(name, value);
    scope.ok(Expr::Nil)
}

pub fn setm(scope: CallScope<'_>) -> Result<Value> {
    scope.assert_arity(1)?;

    let name = scope.at(0).assert_identifier()?;

    scope.env.frames[0].is_macro.insert(name);

    scope.ok(Expr::Nil)
}

/// (defn lambda* (params value))
pub fn lambda(scope: CallScope<'_>) -> Result<Value> {
    scope.assert_arity(2)?;

    let params = scope.at(0).assert_list()?;
    let value = scope.at(1);

    let params = params
        .iter()
        .map(Value::assert_identifier)
        .collect::<Result<_>>()?;

    let meta = scope.env.last_stack().located_at.clone();
    let env = scope.env.clone();

    scope.ok(Closure {
        env,
        meta,
        name: None,
        params,
        value,
    })
}

/// (defn let* (name value))
pub fn let_(scope: CallScope<'_>) -> Result<Value> {
    scope.assert_arity(2)?;

    let name = scope.at(0).assert_identifier()?;
    let value = scope.at(1).eval(scope.env)?;

    scope.env.set(name, value);

    scope.ok(Expr::Nil)
}

/// (defn + (x, args..))
pub fn add(scope: CallScope<'_>) -> Result<Value> {
    let mut result = 0;

    for arg in &scope.args {
        let arg = arg.eval(scope.env)?.assert_number()?;
        result += arg;
    }

    scope.ok(Expr::Int(result))
}

/// (defn - (x, args..))
pub fn sub(scope: CallScope<'_>) -> Result<Value> {
    scope.assert_at_least(1)?;

    let mut result = scope.at(0).assert_number()?;

    for arg in scope.args.iter().skip(1) {
        let arg = arg.eval(scope.env)?.assert_number()?;
        result -= arg;
    }

    scope.ok(Expr::Int(result))
}

/// (defn * (x, args..))
pub fn mul(scope: CallScope<'_>) -> Result<Value> {
    scope.assert_at_least(1)?;

    let mut result = scope.at(0).assert_number()?;

    for arg in scope.args.iter().skip(1) {
        let arg = arg.eval(scope.env)?.assert_number()?;
        result *= arg;
    }

    scope.ok(Expr::Int(result))
}

/// (defn len (indexable))
pub fn len(scope: CallScope<'_>) -> Result<Value> {
    scope.assert_arity(1)?;

    let value = scope.at(0).eval(scope.env)?;

    scope.ok(match &*value.clone().expr() {
        Expr::Cons(..) => Expr::Int(Expr::spine(value).unwrap_or_default().len() as u64),
        Expr::Vector(vector) => Expr::Int(vector.len() as u64),
        Expr::Str(string) => Expr::Int(string.len() as u64),
        _ => return Err(RuntimeError::ExpectedList),
    })
}

/// (defn expand (value))
pub fn expand(scope: CallScope<'_>) -> Result<Value> {
    scope.assert_arity(1)?;

    let value = scope.at(0).eval(scope.env)?;

    Ok(value)
}

/// (defn ' (value))
pub fn quote(scope: CallScope<'_>) -> Result<Value> {
    scope.assert_arity(1)?;
    Ok(scope.at(0))
}

pub fn print(scope: CallScope<'_>) -> Result<Value> {
    let args = scope.args.iter();

    for arg in args {
        match &*arg.0.borrow() {
            Expr::Str(s) => print!("{}", s),
            _ => print!("{}", arg),
        }
    }

    scope.ok(Expr::Nil)
}

pub fn cons(scope: CallScope<'_>) -> Result<Value> {
    scope.assert_arity(2)?;

    let head = scope.at(0).eval(scope.env)?;
    let tail = scope.at(1).eval(scope.env)?;

    scope.ok(Expr::Cons(head, tail))
}

pub fn nil(scope: CallScope<'_>) -> Result<Value> {
    scope.assert_arity(0)?;
    scope.ok(Expr::Nil)
}
