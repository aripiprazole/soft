//! This module defines a bunch of functions that are used by the runtime. All the functions are
//! used by the built-in data types.

use crate::*;

/// (defn call* (name))
pub fn call(scope: CallScope<'_>) -> Result<Value> {
    scope.assert_arity(1)?;

    let arg = scope.at(0);
    let name = arg.assert_identifier()?;

    if let Some(res) = scope.env.get(&name) {
        if scope.env.mode == Mode::Macro && scope.env.global.borrow().is_macro.contains(&name) {
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

    scope.env.global.borrow_mut().variables.insert(name, value);
    scope.ok(Expr::Nil)
}

pub fn setm(scope: CallScope<'_>) -> Result<Value> {
    scope.assert_arity(1)?;

    let name = scope.at(0).assert_identifier()?;

    scope.env.global.borrow_mut().is_macro.insert(name);

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

pub fn defn(scope: CallScope<'_>) -> Result<Value> {
    scope.assert_at_least(3)?;

    let name = scope.at(0).assert_identifier()?;
    let params = scope.at(1).assert_list()?;

    let params = params
        .iter()
        .map(Value::assert_identifier)
        .collect::<Result<_>>()?;

    let meta = scope.env.last_stack().located_at.clone();
    let env = scope.env.clone();

    let block = Value::from_list(scope.args[2..].to_vec());
    let value = Expr::Cons(Expr::Identifier("block".to_string()).to_value(), block).to_value();

    let value = Closure {
        env,
        meta,
        name: Some(name.clone()),
        params,
        value,
    };

    scope
        .env
        .global
        .borrow_mut()
        .variables
        .insert(name, Expr::Closure(value).to_value());

    scope.ok(Expr::Nil)
}

/// (defn let* (name value))
pub fn let_(scope: CallScope<'_>) -> Result<Value> {
    scope.assert_arity(2)?;

    let name = scope.at(0).assert_identifier()?;
    let value = scope.at(1).eval(scope.env)?;

    scope.env.set(name, value);

    scope.ok(Expr::Nil)
}

/// (defn < (x, y))
pub fn less(scope: CallScope<'_>) -> Result<Value> {
    scope.assert_arity(2)?;

    let x = scope.at(0).eval(scope.env)?.assert_number()?;
    let y = scope.at(1).eval(scope.env)?.assert_number()?;

    scope.ok(Expr::Identifier(
        if x < y { "true" } else { "false" }.to_string(),
    ))
}

pub fn if_(scope: CallScope<'_>) -> Result<Value> {
    scope.assert_at_least(2)?;

    let cond = scope.at(0).eval(scope.env)?;

    let value = if cond.is_true() {
        scope.at(1)
    } else {
        scope.at(2)
    };

    value.eval(scope.env)
}

pub fn block(scope: CallScope<'_>) -> Result<Value> {
    scope.env.add_local_stack();

    let mut result = Expr::Nil.to_value();

    for arg in scope.args {
        result = arg.eval(scope.env)?;
    }

    scope.env.pop_stack();

    Ok(result)
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

    let mut result = scope.at(0).eval(scope.env)?.assert_number()?;

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
        _ => return Err(RuntimeError::ExpectedList(value.to_string())),
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
        match &*arg.eval(scope.env)?.0.borrow() {
            Expr::Str(s) => print!("{}", s),
            a => print!("{}", a),
        }
    }

    println!();

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
