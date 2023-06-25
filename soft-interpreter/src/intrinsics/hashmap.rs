use fxhash::FxHashMap;

use crate::{
    error::{Result, RuntimeError},
    value::{CallScope, Expr, Trampoline},
};

pub fn hash_map(scope: CallScope<'_>) -> Result<Trampoline> {
    let tuples = scope
        .args
        .into_iter()
        .map(|x| x.assert_fixed_size_list(2))
        .collect::<Result<Vec<_>>>()?;

    let mut map = FxHashMap::default();

    for tuple in tuples {
        let key_val = tuple[0].clone().run(scope.env)?;
        let key = key_val.stringify();
        let value = tuple[1].clone().run(scope.env)?;

        map.insert(key, (key_val, value));
    }

    Ok(Trampoline::returning(Expr::HashMap(map)))
}

pub fn hash_map_get(scope: CallScope<'_>) -> Result<Trampoline> {
    scope.assert_arity(2)?;

    let map = scope.at(0).run(scope.env)?;
    let key = scope.at(1).run(scope.env)?.stringify();

    let borrow = map.borrow_mut();

    match borrow.kind {
        Expr::HashMap(ref map) => {
            let value = map
                .get(&key)
                .map(|x| x.1.clone())
                .unwrap_or_else(|| Expr::Nil.into());

            Ok(Trampoline::returning(value))
        }
        _ => Err(RuntimeError::from("expected hash-map")),
    }
}

pub fn hash_map_set(scope: CallScope<'_>) -> Result<Trampoline> {
    scope.assert_arity(3)?;

    let map = scope.at(0).run(scope.env)?;
    let key = scope.at(1).run(scope.env)?;
    let value = scope.at(2).run(scope.env)?;

    let borrow = map.borrow_mut();

    match borrow.kind {
        Expr::HashMap(ref mut map) => {
            map.insert(key.stringify(), (key, value));
            Ok(Trampoline::returning(Expr::Nil))
        }
        _ => Err(RuntimeError::from("expected hash-map")),
    }
}

pub fn hash_map_keys(scope: CallScope<'_>) -> Result<Trampoline> {
    scope.assert_arity(1)?;

    let map = scope.at(0).run(scope.env)?;

    let borrow = map.borrow_mut();

    match borrow.kind {
        Expr::HashMap(ref map) => {
            let keys = map.values().map(|x| x.0.clone()).collect::<Vec<_>>();

            Ok(Trampoline::returning(Expr::Vector(keys)))
        }
        _ => Err(RuntimeError::from("expected hash-map")),
    }
}

pub fn hash_map_vals(scope: CallScope<'_>) -> Result<Trampoline> {
    scope.assert_arity(1)?;

    let map = scope.at(0).run(scope.env)?;

    let borrow = map.borrow_mut();

    match borrow.kind {
        Expr::HashMap(ref map) => {
            let values = map.values().map(|x| x.1.clone()).collect::<Vec<_>>();

            Ok(Trampoline::returning(Expr::Vector(values)))
        }
        _ => Err(RuntimeError::from("expected hash-map")),
    }
}

pub fn hash_map_len(scope: CallScope<'_>) -> Result<Trampoline> {
    scope.assert_arity(1)?;

    let map = scope.at(0).run(scope.env)?;

    let borrow = map.borrow_mut();

    match borrow.kind {
        Expr::HashMap(ref map) => Ok(Trampoline::returning(Expr::Int(map.len() as i64))),
        _ => Err(RuntimeError::from("expected hash-map")),
    }
}
