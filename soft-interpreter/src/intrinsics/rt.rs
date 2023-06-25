use crate::error::{Result, RuntimeError};
use crate::value::{CallScope, Closure, Expr, Function, Param, Spanned, Trampoline, Value};

/// if : bool -> a -> a -> a
pub fn if_(scope: CallScope<'_>) -> Result<Trampoline> {
    scope.assert_arity(3)?;

    let condition = scope.at(0).run(scope.env)?.is_true();
    let consequent = scope.at(1);
    let alternative = scope.at(2);

    let expr = if condition { consequent } else { alternative };

    Ok(Trampoline::eval(expr))
}

/// block : a... -> a
pub fn block(scope: CallScope<'_>) -> Result<Trampoline> {
    scope.assert_at_least(1)?;

    let last = scope.args.last().unwrap();

    for arg in scope.args.iter().take(scope.args.len() - 1).cloned() {
        arg.run(scope.env)?;
    }

    Ok(Trampoline::eval(last.clone()))
}

/// expand : expr -> expr
pub fn expand(scope: CallScope<'_>) -> Result<Trampoline> {
    scope.assert_arity(1)?;

    let value = scope.at(0).run(scope.env)?;
    let value = value.expand(scope.env)?;

    Ok(Trampoline::Return(value))
}

/// quote : expr -> expr
pub fn quote(scope: CallScope<'_>) -> Result<Trampoline> {
    scope.assert_arity(1)?;

    Ok(Trampoline::returning(scope.at(0)))
}

/// eval : expr -> expr
pub fn eval(scope: CallScope<'_>) -> Result<Trampoline> {
    scope.assert_arity(1)?;
    let expr = scope.at(0);
    Ok(Trampoline::Eval(expr))
}

/// fn* : id -> a -> fn*
pub fn fn_(scope: CallScope<'_>) -> Result<Trampoline> {
    scope.assert_arity(3)?;

    let name = scope.at(0).assert_identifier()?;
    let params = scope.at(1).assert_list()?;
    let value = scope.at(2);

    let params = params
        .iter()
        .map(Value::assert_identifier)
        .collect::<Result<Vec<_>>>()?;

    let mut iter = params.into_iter();

    let mut params = vec![];

    while let Some(param) = iter.next() {
        if param == "&rest" {
            let Some(param) = iter.next() else {
                return Err(RuntimeError::ExpectedIdentifier("nothing".to_string()));
            };
            params.push(Param::Variadic(param));
        } else if param == "&optional" {
            let Some(param) = iter.next() else {
                return Err(RuntimeError::ExpectedIdentifier("nothing".to_string()));
            };
            params.push(Param::Optional(param, Value::from(Expr::Nil)))
        } else {
            params.push(Param::Required(param));
        }
    }

    let mut frame = scope.env.last_frame().clone();
    frame.name = Some(name.clone());

    let closure = Closure {
        name: Some(name),
        frame,
        params,
        expr: value,
    };

    Ok(Trampoline::returning(Spanned::new(
        Expr::Function(Function::Closure(closure)),
        scope.env.location.clone().into(),
    )))
}

/// nil? : a -> bool
pub fn is_nil(scope: CallScope<'_>) -> Result<Trampoline> {
    scope.assert_arity(1)?;

    let value = scope.at(0).is_nil();

    let value = if value {
        Expr::Id("true".to_string())
    } else {
        Expr::Nil
    };

    Ok(Trampoline::Return(value.into()))
}

/// vec? : a -> bool
pub fn is_vec(scope: CallScope<'_>) -> Result<Trampoline> {
    scope.assert_arity(1)?;

    let value = scope.at(0).is_vec();

    let value = if value {
        Expr::Id("true".to_string())
    } else {
        Expr::Nil
    };

    Ok(Trampoline::Return(value.into()))
}

/// int? : a -> bool
pub fn is_int(scope: CallScope<'_>) -> Result<Trampoline> {
    scope.assert_arity(1)?;

    let value = scope.at(0).is_int();

    let value = if value {
        Expr::Id("true".to_string())
    } else {
        Expr::Nil
    };

    Ok(Trampoline::Return(value.into()))
}

/// atom? : a -> bool
pub fn is_atom(scope: CallScope<'_>) -> Result<Trampoline> {
    scope.assert_arity(1)?;

    let value = scope.at(0).is_atom();

    let value = if value {
        Expr::Id("true".to_string())
    } else {
        Expr::Nil
    };

    Ok(Trampoline::Return(value.into()))
}

pub fn is_function(scope: CallScope<'_>) -> Result<Trampoline> {
    scope.assert_arity(1)?;

    let value = scope.at(0).is_function();

    let value = if value {
        Expr::Id("true".to_string())
    } else {
        Expr::Nil
    };

    Ok(Trampoline::Return(value.into()))
}

pub fn is_error(scope: CallScope<'_>) -> Result<Trampoline> {
    scope.assert_arity(1)?;

    let value = scope.at(0).is_error();

    let value = if value {
        Expr::Id("true".to_string())
    } else {
        Expr::Nil
    };

    Ok(Trampoline::Return(value.into()))
}

pub fn to_string(scope: CallScope<'_>) -> Result<Trampoline> {
    scope.assert_arity(1)?;

    let value = scope.at(0).run(scope.env)?.stringify();

    Ok(Trampoline::returning(Expr::Str(value)))
}

pub fn to_atom(scope: CallScope<'_>) -> Result<Trampoline> {
    scope.assert_arity(1)?;

    let value = scope.at(0).run(scope.env)?.stringify();

    Ok(Trampoline::returning(Expr::Id(value)))
}

pub fn to_int(scope: CallScope<'_>) -> Result<Trampoline> {
    scope.assert_arity(1)?;

    let value = scope.at(0).run(scope.env)?.stringify();

    let value = value.parse::<i64>().unwrap_or_default();

    Ok(Trampoline::returning(Expr::Int(value)))
}
