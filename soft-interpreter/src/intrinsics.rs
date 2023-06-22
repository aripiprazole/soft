use crate::error::Result;
use crate::value::{CallScope, Closure, Expr, ExprKind, Function, Trampoline, Value};

pub fn lambda(scope: CallScope<'_>) -> Result<Trampoline> {
    scope.assert_arity(2)?;

    let params = scope.at(0).assert_list()?;
    let value = scope.at(1);

    let params = params
        .iter()
        .map(Value::assert_identifier)
        .collect::<Result<_>>()?;

    let location = scope.env.last_frame().located_at.clone();
    let frame = scope.env.last_frame().clone();

    let closure = Closure {
        frame,
        params,
        location,
        expr: value,
    };

    Ok(Trampoline::Return(
        Expr::new(ExprKind::Function(Function::Closure(closure)), None).into(),
    ))
}

pub fn add(scope: CallScope<'_>) -> Result<Trampoline> {
    scope.assert_at_least(1)?;

    let mut result = 0;

    for arg in scope.args {
        let arg = arg.run(scope.env)?.assert_number()?;
        result += arg;
    }

    Ok(Trampoline::Return(
        Expr::new(ExprKind::Int(result), None).into(),
    ))
}

pub fn sub(scope: CallScope<'_>) -> Result<Trampoline> {
    scope.assert_at_least(1)?;

    let mut result = scope.at(0).run(scope.env)?.assert_number()?;

    for arg in scope.args.iter().skip(1) {
        let arg = arg.clone().run(scope.env)?.assert_number()?;
        result -= arg;
    }

    Ok(Trampoline::Return(
        Expr::new(ExprKind::Int(result), None).into(),
    ))
}

pub fn set(scope: CallScope<'_>) -> Result<Trampoline> {
    scope.assert_arity(2)?;

    let name = scope.at(0).assert_identifier()?;
    let value = scope.at(1).run(scope.env)?;

    scope.env.insert_global(name, value);

    Ok(Trampoline::Return(Expr::new(ExprKind::Nil, None).into()))
}

pub fn if_(scope: CallScope<'_>) -> Result<Trampoline> {
    scope.assert_arity(3)?;

    let condition = scope.at(0).run(scope.env)?.is_true();
    let consequent = scope.at(1);
    let alternative = scope.at(2);

    let expr = if condition { consequent } else { alternative };

    Ok(Trampoline::Eval(expr))
}

pub fn less_than(scope: CallScope<'_>) -> Result<Trampoline> {
    scope.assert_arity(2)?;

    let left = scope.at(0).run(scope.env)?.assert_number()?;
    let right = scope.at(1).run(scope.env)?.assert_number()?;

    let value = if left < right {
        ExprKind::Id("true".to_string())
    } else {
        ExprKind::Nil
    };

    Ok(Trampoline::Return(Expr::new(value, None).into()))
}
