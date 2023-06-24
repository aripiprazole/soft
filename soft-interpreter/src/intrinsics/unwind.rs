use crate::error::{Result, RuntimeError};
use crate::value::{CallScope, Expr, Trampoline, Value};

/// throw : a -> error a
pub fn throw(scope: CallScope<'_>) -> Result<Trampoline> {
    scope.assert_arity(1)?;

    Err(RuntimeError::UserError(scope.at(0).run(scope.env)?))
}

/// try : a -> (error a -> b) -> b
pub fn try_(scope: CallScope<'_>) -> Result<Trampoline> {
    scope.assert_arity(2)?;

    let try_expr = scope.at(0);
    let catch_expr = scope.at(1);

    let catch_expr = catch_expr.assert_list()?;
    if catch_expr.len() < 2 {
        return Err(RuntimeError::CatchRequiresTwoArgs);
    }

    let catch_name = catch_expr[0].assert_identifier()?;
    let catch_body = catch_expr[1].clone();

    scope.env.enable_catching();
    let value = try_expr.run(scope.env);

    let err = match value {
        Ok(value) => {
            scope.env.disable_catching();

            return Ok(Trampoline::returning(value));
        }
        Err(err @ RuntimeError::UserError(..)) => err,
        Err(err) => return Err(err),
    };
    let stack = scope.env.unwind();

    scope.env.disable_catching();

    let err = Value::from(Expr::Err(err, stack));
    let catch_frame = scope.env.last_frame();
    catch_frame.insert(catch_name, err);

    Ok(Trampoline::returning(catch_body.run(scope.env)?))
}

/// err/print-stack : error a -> error a
pub fn err_print_stack(scope: CallScope<'_>) -> Result<Trampoline> {
    scope.assert_arity(1)?;

    let value = scope.at(0).run(scope.env)?;

    let (err, stack) = value.assert_error()?;

    println!("tracing runtime error: {err}");
    for frame in stack.iter() {
        let name = frame.name.clone().unwrap_or("unknown".to_string());
        println!("  in {} at {}", name, frame.location);
    }

    Ok(Trampoline::returning(value))
}
