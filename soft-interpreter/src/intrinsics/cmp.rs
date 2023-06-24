use crate::error::Result;
use crate::value::{CallScope, Expr, Trampoline};

pub fn eq(scope: CallScope<'_>) -> Result<Trampoline> {
    scope.assert_arity(2)?;

    let left = scope.at(0).run(scope.env)?;
    let right = scope.at(1).run(scope.env)?;

    let tt = Expr::Id("true".to_string());
    let ff = Expr::Nil;

    let result = match (&left.kind, &right.kind) {
        (Expr::Int(left), Expr::Int(right)) => left == right,
        (Expr::Id(left), Expr::Id(right)) => left == right,
        (Expr::Str(left), Expr::Str(right)) => left == right,
        (Expr::Nil, Expr::Nil) => true,
        _ => false,
    };

    Ok(Trampoline::returning(if result { tt } else { ff }))
}

pub fn greater_than(scope: CallScope<'_>) -> Result<Trampoline> {
    scope.assert_arity(2)?;

    let left = scope.at(0).run(scope.env)?.assert_number()?;
    let right = scope.at(1).run(scope.env)?.assert_number()?;

    let value = if left > right {
        Expr::Id("true".to_string())
    } else {
        Expr::Nil
    };

    Ok(Trampoline::returning(value))
}

pub fn less_than(scope: CallScope<'_>) -> Result<Trampoline> {
    scope.assert_arity(2)?;

    let left = scope.at(0).run(scope.env)?.assert_number()?;
    let right = scope.at(1).run(scope.env)?.assert_number()?;

    let value = if left < right {
        Expr::Id("true".to_string())
    } else {
        Expr::Nil
    };

    Ok(Trampoline::returning(value))
}
