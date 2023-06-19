//! This module defines a bunch of functions that are used by the runtime. All the functions are
//! used by the built-in data types.

use crate::*;

pub fn call(scope: CallScope<'_>) -> Result<Value> {
    scope.assert_arity(1)?;

    let arg = scope.at(0);
    let name = arg.assert_identifier()?;

    if let Some(res) = scope.env.get(&name) {
        Ok(res)
    } else {
        Err(RuntimeError::UndefinedName(name))
    }
}

pub fn set(scope: CallScope<'_>) -> Result<Value> {
    scope.assert_arity(2)?;

    let name = scope.at(0).assert_identifier()?;
    let value = scope.at(1).eval(scope.env)?;

    scope.env.frames[0].variables.insert(name, value);
    scope.ok(Expr::Nil)
}

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

pub fn let_(scope: CallScope<'_>) -> Result<Value> {
    scope.assert_arity(2)?;

    let name = scope.at(0).assert_identifier()?;
    let value = scope.at(1).eval(scope.env)?;

    scope.env.set(name, value);

    scope.ok(Expr::Nil)
}

pub fn add(scope: CallScope<'_>) -> Result<Value> {
    let mut result = 0;

    for arg in &scope.args {
        let arg = arg.eval(scope.env)?.assert_number()?;
        result += arg;
    }

    scope.ok(Expr::Int(result))
}

pub fn quote(scope: CallScope<'_>) -> Result<Value> {
    scope.assert_arity(1)?;
    Ok(scope.at(0))
}
