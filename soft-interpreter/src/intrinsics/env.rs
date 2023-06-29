use crate::error::Result;
use crate::value::{CallScope, Expr, Function, Trampoline};

/// set : id -> a -> nil
pub fn letm(scope: CallScope<'_>) -> Result<Trampoline> {
    scope.assert_arity(2)?;

    let name = scope.at(0).assert_identifier()?;
    let value = scope.at(1).run(scope.env)?;

    scope.env.insert(name, value);

    Ok(Trampoline::returning(Expr::Nil))
}

/// set : id -> a -> nil
pub fn letrec(scope: CallScope<'_>) -> Result<Trampoline> {
    scope.assert_arity(2)?;

    let name = scope.at(0).assert_identifier()?;
    let value = scope.at(1).run(scope.env)?;

    let itself = value.clone();

    if let Expr::Function(Function::Closure(ref mut x)) = value.borrow_mut().kind {
        x.frame.insert(name.clone(), itself.clone())
    }

    scope.env.insert(name, itself);

    Ok(Trampoline::returning(Expr::Nil))
}

/// set : id -> a -> nil
pub fn set(scope: CallScope<'_>) -> Result<Trampoline> {
    scope.assert_arity(2)?;

    let name = scope.at(0).assert_identifier()?;
    let value = scope.at(1).run(scope.env)?;

    scope.env.insert_global(name, value, false)?;

    Ok(Trampoline::returning(Expr::Nil))
}

/// setm : id -> a -> nil
pub fn setm(scope: CallScope<'_>) -> Result<Trampoline> {
    scope.assert_arity(2)?;

    let name = scope.at(0).assert_identifier()?;
    let value = scope.at(1).run(scope.env)?;

    scope.env.insert_global(name, value, true)?;

    Ok(Trampoline::returning(Expr::Nil))
}
