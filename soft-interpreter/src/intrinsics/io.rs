use std::path::PathBuf;

use crate::error::{Result, RuntimeError};
use crate::reader;
use crate::value::{CallScope, Expr, Trampoline, Value};

/// print : a... -> nil
pub fn print(scope: CallScope<'_>) -> Result<Trampoline> {
    for arg in scope.args.iter() {
        print!("{}", arg.clone().run(scope.env)?);
    }

    println!();

    Ok(Trampoline::returning(Expr::Nil))
}

/// import : string -> nil
pub fn import(scope: CallScope<'_>) -> Result<Trampoline> {
    scope.assert_arity(1)?;

    let mut cwd = scope
        .env
        .location
        .file
        .clone()
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap());

    let path = scope.at(0).assert_string()?;
    cwd.pop();

    cwd.push(path);

    let cwd = cwd.canonicalize().unwrap();

    if scope.env.imported_files.contains(&cwd) {
        return Ok(Trampoline::returning(Expr::Nil));
    } else {
        scope.env.imported_files.insert(cwd.clone());
    }

    let cwd = pathdiff::diff_paths(cwd, std::env::current_dir().unwrap()).unwrap();

    let Ok(contents) = std::fs::read_to_string(&cwd) else {
        let msg = format!("cannot find file '{}'", cwd.display());
        return Err(RuntimeError::UserError(Value::from(Expr::Str(msg))))
    };

    let values = reader::read(&contents, Some(cwd.to_str().unwrap().to_owned()))?;

    values
        .into_iter()
        .try_fold(Expr::Nil.into(), |_, next| next.run(scope.env))?;

    Ok(Trampoline::returning(Expr::Nil))
}
