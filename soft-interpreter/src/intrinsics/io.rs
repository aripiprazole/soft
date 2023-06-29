use std::path::PathBuf;

use crate::error::{Result, RuntimeError};
use crate::reader;
use crate::value::{CallScope, Expr, Trampoline, Value};

/// print : a... -> nil
pub fn print(scope: CallScope<'_>) -> Result<Trampoline> {
    for arg in scope.args.iter() {
        print!("{}", arg.clone().run(scope.env)?.stringify());
    }

    Ok(Trampoline::returning(Expr::Nil))
}

pub fn flush(scope: CallScope<'_>) -> Result<Trampoline> {
    use std::io::Write;

    scope.assert_arity(0)?;

    let Ok(_) = std::io::stdout().flush() else {
        return Err(RuntimeError::from("cannot flush stdout"));
    };

    Ok(Trampoline::returning(Expr::Nil))
}

pub fn read(scope: CallScope<'_>) -> Result<Trampoline> {
    scope.assert_arity(0)?;

    let mut buffer = String::new();

    let Ok(_) = std::io::stdin().read_line(&mut buffer) else {
        return Err(RuntimeError::from("cannot read from stdin"));
    };

    buffer.pop();

    Ok(Trampoline::returning(Expr::Str(buffer)))
}

pub fn read_file(scope: CallScope<'_>) -> Result<Trampoline> {
    scope.assert_arity(1)?;

    let path = scope.at(0).assert_string()?;

    let Ok(contents) = std::fs::read_to_string(path) else {
        return Err(RuntimeError::from("cannot read file"));
    };

    Ok(Trampoline::returning(Expr::Str(contents)))
}

pub fn parse(scope: CallScope<'_>) -> Result<Trampoline> {
    scope.assert_arity(2)?;

    let contents = scope.at(0).assert_string()?;
    let path = scope.at(1).assert_string()?;

    let values = reader::read(&contents, Some(path))?;

    Ok(Trampoline::returning(Value::from_iter(
        values.into_iter(),
        None,
    )))
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
