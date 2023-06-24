use crate::error::Result;
use crate::value::{CallScope, Expr, Trampoline};

pub fn string_length(scope: CallScope<'_>) -> Result<Trampoline> {
    scope.assert_arity(1)?;

    let value = scope.at(0).run(scope.env)?.assert_string()?;

    let value = value.len();

    Ok(Trampoline::returning(Expr::Int(value as i64)))
}

pub fn string_slice(scope: CallScope<'_>) -> Result<Trampoline> {
    scope.assert_arity(3)?;

    let value = scope.at(0).run(scope.env)?.assert_string()?;
    let start = scope.at(1).run(scope.env)?.assert_number()? as usize;
    let end = scope.at(2).run(scope.env)?.assert_number()? as usize;

    let value = &value[start..end];

    Ok(Trampoline::returning(Expr::Str(value.to_string())))
}

pub fn string_concat(scope: CallScope<'_>) -> Result<Trampoline> {
    let args = scope
        .args
        .into_iter()
        .map(|v| v.run(scope.env))
        .collect::<Result<Vec<_>>>()?;

    let strings = args
        .iter()
        .map(|v| v.assert_string())
        .collect::<Result<Vec<_>>>()?;

    let value = strings.join("");

    Ok(Trampoline::returning(Expr::Str(value)))
}

pub fn string_split(scope: CallScope<'_>) -> Result<Trampoline> {
    scope.assert_arity(2)?;

    let value = scope.at(0).run(scope.env)?.assert_string()?;
    let sep = scope.at(1).run(scope.env)?.assert_string()?;

    let value = value
        .split(&sep)
        .map(|s| Expr::Str(s.to_string()).into())
        .collect();

    Ok(Trampoline::returning(Expr::Vector(value)))
}
