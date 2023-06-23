use crate::error::{Result, RuntimeError};
use crate::value::{CallScope, Closure, Expr, ExprKind, Function, Trampoline, Value};

pub fn lambda(scope: CallScope<'_>) -> Result<Trampoline> {
    scope.assert_arity(3)?;

    let name = scope.at(0).assert_identifier()?;
    let params = scope.at(1).assert_list()?;
    let value = scope.at(2);

    let params = params
        .iter()
        .map(Value::assert_identifier)
        .collect::<Result<_>>()?;

    let mut frame = scope.env.last_frame().clone();

    frame.name = Some(name.clone());

    let closure = Closure {
        name: Some(name),
        frame,
        params,
        expr: value,
    };

    Ok(Trampoline::Return(
        Expr::new(
            ExprKind::Function(Function::Closure(closure)),
            scope.env.location.clone().into(),
        )
        .into(),
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

    scope.env.insert_global(name, value, false);

    Ok(Trampoline::Return(Expr::new(ExprKind::Nil, None).into()))
}

pub fn setm(scope: CallScope<'_>) -> Result<Trampoline> {
    scope.assert_arity(2)?;

    let name = scope.at(0).assert_identifier()?;
    let value = scope.at(1).run(scope.env)?;

    scope.env.insert_global(name, value, true);

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

pub fn expand(scope: CallScope<'_>) -> Result<Trampoline> {
    scope.assert_arity(1)?;

    let value = scope.at(0).run(scope.env)?;
    let value = value.expand(scope.env)?;

    Ok(Trampoline::Return(value))
}

pub fn quote(scope: CallScope<'_>) -> Result<Trampoline> {
    scope.assert_arity(1)?;

    Ok(Trampoline::Return(scope.at(0)))
}

pub fn is_cons(scope: CallScope<'_>) -> Result<Trampoline> {
    scope.assert_arity(1)?;

    let value = scope.at(0).run(scope.env)?;

    let value = if value.kind.is_cons() {
        ExprKind::Id("true".to_string())
    } else {
        ExprKind::Nil
    };

    Ok(Trampoline::Return(Expr::new(value, None).into()))
}

pub fn print(scope: CallScope<'_>) -> Result<Trampoline> {
    for arg in scope.args.iter() {
        print!("{}", arg.clone().run(scope.env)?);
    }
    Ok(Trampoline::Return(Expr::new(ExprKind::Nil, None).into()))
}

pub fn eq(scope: CallScope<'_>) -> Result<Trampoline> {
    scope.assert_arity(2)?;

    let left = scope.at(0).run(scope.env)?;
    let right = scope.at(1).run(scope.env)?;

    let tt = Expr::new(ExprKind::Id("true".to_string()), None).into();
    let ff = Expr::new(ExprKind::Nil, None).into();

    let result = match (&left.kind, &right.kind) {
        (ExprKind::Int(left), ExprKind::Int(right)) => left == right,
        (ExprKind::Id(left), ExprKind::Id(right)) => left == right,
        (ExprKind::Str(left), ExprKind::Str(right)) => left == right,
        (ExprKind::Nil, ExprKind::Nil) => true,
        _ => false,
    };

    Ok(Trampoline::Return(if result { tt } else { ff }))
}

pub fn head(scope: CallScope<'_>) -> Result<Trampoline> {
    scope.assert_arity(1)?;

    let value = scope.at(0).run(scope.env)?;

    match &value.kind {
        ExprKind::Cons(head, _) => Ok(Trampoline::Return(head.clone())),
        _ => Err(RuntimeError::ExpectedList(value.to_string())),
    }
}

pub fn tail(scope: CallScope<'_>) -> Result<Trampoline> {
    scope.assert_arity(1)?;

    let value = scope.at(0).run(scope.env)?;

    match &value.kind {
        ExprKind::Cons(_, tail) => Ok(Trampoline::Return(tail.clone())),
        _ => Err(RuntimeError::ExpectedList(value.to_string())),
    }
}

pub fn cons(scope: CallScope<'_>) -> Result<Trampoline> {
    scope.assert_arity(2)?;

    let head = scope.at(0).run(scope.env)?;
    let tail = scope.at(1).run(scope.env)?;

    Ok(Trampoline::Return(
        Expr::new(ExprKind::Cons(head, tail), None).into(),
    ))
}

pub fn list(scope: CallScope<'_>) -> Result<Trampoline> {
    let mut result = Expr::new(ExprKind::Nil, None);

    for arg in scope.args.into_iter().rev() {
        let arg = arg.run(scope.env)?;
        result = Expr::new(ExprKind::Cons(arg, result.into()), None);
    }

    Ok(Trampoline::Return(result.into()))
}
