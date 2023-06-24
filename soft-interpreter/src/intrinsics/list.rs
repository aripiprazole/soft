use crate::error::{Result, RuntimeError};
use crate::value::{CallScope, Expr, Spanned, Trampoline};

/// idx : number -> list a -> a
pub fn idx(scope: CallScope<'_>) -> Result<Trampoline> {
    scope.assert_arity(2)?;

    let idx = scope.at(0).run(scope.env)?.assert_number()?;
    let list = scope.at(1).run(scope.env)?.assert_list()?;

    let Some(value) = list.get(idx as usize).cloned() else {
        return Err(RuntimeError::from(format!(
            "Index {} out of bounds for list of length {}",
            idx,
            list.len()
        )));
    };

    Ok(Trampoline::returning(value))
}

/// cons? : a -> bool
pub fn is_cons(scope: CallScope<'_>) -> Result<Trampoline> {
    scope.assert_arity(1)?;

    let value = scope.at(0).run(scope.env)?;

    let value = if value.kind.is_cons() {
        Expr::Id("true".to_string())
    } else {
        Expr::Nil
    };

    Ok(Trampoline::returning(value))
}

/// head : list a -> a
pub fn head(scope: CallScope<'_>) -> Result<Trampoline> {
    scope.assert_arity(1)?;

    let value = scope.at(0).run(scope.env)?;

    match &value.kind {
        Expr::Cons(head, _) => Ok(Trampoline::Return(head.clone())),
        _ => Err(RuntimeError::ExpectedList(value.to_string())),
    }
}

/// tail : list a -> list a
pub fn tail(scope: CallScope<'_>) -> Result<Trampoline> {
    scope.assert_arity(1)?;

    let value = scope.at(0).run(scope.env)?;

    match &value.kind {
        Expr::Cons(_, tail) => Ok(Trampoline::returning(tail.clone())),
        _ => Err(RuntimeError::ExpectedList(value.to_string())),
    }
}

/// cons : a -> list a -> list a
pub fn cons(scope: CallScope<'_>) -> Result<Trampoline> {
    scope.assert_arity(2)?;

    let head = scope.at(0).run(scope.env)?;
    let tail = scope.at(1).run(scope.env)?;

    Ok(Trampoline::returning(Expr::Cons(head, tail)))
}

/// list : a... -> list a
pub fn list(scope: CallScope<'_>) -> Result<Trampoline> {
    let mut result = Spanned::new(Expr::Nil, None);

    for arg in scope.args.into_iter().rev() {
        let arg = arg.run(scope.env)?;
        result = Expr::Cons(arg, result.into()).into();
    }

    Ok(Trampoline::returning(result))
}
