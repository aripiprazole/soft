//! This module defines a bunch of functions that are used by the runtime. All the functions are
//! used by the built-in data types.

use crate::{CallScope, Result, RuntimeError, Value};

pub fn call(scope: CallScope<'_>) -> Result<Value> {
    scope.assert_size(1)?;
    let arg = scope.args[0].clone();
    let id = arg.assert_identifier()?;

    if let Some(res) = scope.env.get(&id) {
        Ok(res)
    } else {
        Err(RuntimeError::UndefinedName(id))
    }
}
