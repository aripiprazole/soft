use crate::{
    error::{Result, RuntimeError},
    value::{CallScope, Expr, Trampoline},
};

pub fn vec_index(scope: CallScope<'_>) -> Result<Trampoline> {
    scope.assert_arity(2)?;

    let vec = scope.at(0).run(scope.env)?;
    let idx = scope.at(1).run(scope.env)?;

    let idx = idx.assert_number()?;

    let value = match &vec.kind {
        Expr::Vector(vec) => vec
            .get(idx as usize)
            .ok_or_else(|| RuntimeError::from("index out of bounds"))
            .cloned()?,
        _ => return Err(RuntimeError::from("expected vec")),
    };

    Ok(Trampoline::returning(value))
}

pub fn vec_set(scope: CallScope<'_>) -> Result<Trampoline> {
    scope.assert_arity(3)?;

    let vec = scope.at(0).run(scope.env)?;
    let idx = scope.at(1).run(scope.env)?;
    let value = scope.at(2).run(scope.env)?;

    let idx = idx.assert_number()?;

    match vec.borrow_mut().kind {
        Expr::Vector(ref mut vec) => {
            vec[idx as usize] = value;
            Ok(Trampoline::returning(Expr::Nil))
        }
        _ => Err(RuntimeError::from("expected vec")),
    }
}

pub fn vec_push(scope: CallScope<'_>) -> Result<Trampoline> {
    scope.assert_arity(2)?;

    let vec = scope.at(0).run(scope.env)?;
    let value = scope.at(1).run(scope.env)?;

    let borrow = vec.borrow_mut();

    match borrow.kind {
        Expr::Vector(ref mut vec) => {
            vec.push(value);
            Ok(Trampoline::returning(Expr::Nil))
        }
        _ => Err(RuntimeError::from("expected vec")),
    }
}

pub fn vec_pop(scope: CallScope<'_>) -> Result<Trampoline> {
    scope.assert_arity(1)?;

    let vec = scope.at(0).run(scope.env)?;

    let borrow = vec.borrow_mut();

    match borrow.kind {
        Expr::Vector(ref mut vec) => {
            Ok(Trampoline::returning(vec.pop().unwrap_or(Expr::Nil.into())))
        }
        _ => Err(RuntimeError::from("expected vec")),
    }
}

pub fn vec_len(scope: CallScope<'_>) -> Result<Trampoline> {
    scope.assert_arity(1)?;

    let vec = scope.at(0).run(scope.env)?;

    let borrow = vec.borrow_mut();

    match borrow.kind {
        Expr::Vector(ref vec) => Ok(Trampoline::returning(Expr::Int(vec.len() as i64))),
        _ => Err(RuntimeError::from("expected vec")),
    }
}

pub fn vec(scope: CallScope<'_>) -> Result<Trampoline> {
    let args = scope
        .args
        .into_iter()
        .map(|arg| arg.run(scope.env))
        .collect::<Result<Vec<_>>>()?;

    Ok(Trampoline::returning(Expr::Vector(args)))
}
