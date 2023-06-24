use crate::error::Result;
use crate::value::{CallScope, Expr, Trampoline};

/// + : a -> a -> a
pub fn add(scope: CallScope<'_>) -> Result<Trampoline> {
    scope.assert_at_least(1)?;

    let mut result = 0;

    for arg in scope.args {
        let arg = arg.run(scope.env)?.assert_number()?;
        result += arg;
    }

    Ok(Trampoline::returning(Expr::Int(result)))
}

/// - : a -> a -> a
pub fn sub(scope: CallScope<'_>) -> Result<Trampoline> {
    scope.assert_at_least(1)?;

    let mut result = scope.at(0).run(scope.env)?.assert_number()?;

    for arg in scope.args.iter().skip(1) {
        let arg = arg.clone().run(scope.env)?.assert_number()?;
        result -= arg;
    }

    Ok(Trampoline::returning(Expr::Int(result)))
}

/// + : a -> a -> a
pub fn mul(scope: CallScope<'_>) -> Result<Trampoline> {
    scope.assert_at_least(1)?;

    let mut result = 1;

    for arg in scope.args {
        let arg = arg.run(scope.env)?.assert_number()?;
        result *= arg;
    }

    Ok(Trampoline::returning(Expr::Int(result)))
}

/// + : a -> a -> a
pub fn div(scope: CallScope<'_>) -> Result<Trampoline> {
    scope.assert_at_least(1)?;

    let mut result = scope.at(0).run(scope.env)?.assert_number()?;

    for arg in scope.args.iter().skip(1) {
        let arg = arg.clone().run(scope.env)?.assert_number()?;
        result /= arg;
    }

    Ok(Trampoline::returning(Expr::Int(result)))
}
