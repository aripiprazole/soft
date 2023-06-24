use std::path::PathBuf;

use crate::error::{Result, RuntimeError};
use crate::reader;
use crate::value::{CallScope, Closure, Expr, Function, Spanned, Trampoline, Value};

pub fn lambda(scope: CallScope<'_>) -> Result<Trampoline> {
    scope.assert_arity(3)?;

    let name = scope.at(0).assert_identifier()?;
    let params = scope.at(1).assert_list()?;
    let value = scope.at(2);

    let params = params
        .iter()
        .map(Value::assert_identifier)
        .collect::<Result<_>>()?;

    let mut frame = scope.env.last_frame().clone();
    frame.name = Some(name.clone());

    let closure = Closure {
        name: Some(name),
        frame,
        params,
        expr: value,
    };

    Ok(Trampoline::returning(Spanned::new(
        Expr::Function(Function::Closure(closure)),
        scope.env.location.clone().into(),
    )))
}

pub fn add(scope: CallScope<'_>) -> Result<Trampoline> {
    scope.assert_at_least(1)?;

    let mut result = 0;

    for arg in scope.args {
        let arg = arg.run(scope.env)?.assert_number()?;
        result += arg;
    }

    Ok(Trampoline::returning(Expr::Int(result)))
}

pub fn sub(scope: CallScope<'_>) -> Result<Trampoline> {
    scope.assert_at_least(1)?;

    let mut result = scope.at(0).run(scope.env)?.assert_number()?;

    for arg in scope.args.iter().skip(1) {
        let arg = arg.clone().run(scope.env)?.assert_number()?;
        result -= arg;
    }

    Ok(Trampoline::returning(Expr::Int(result)))
}

pub fn set(scope: CallScope<'_>) -> Result<Trampoline> {
    scope.assert_arity(2)?;

    let name = scope.at(0).assert_identifier()?;
    let value = scope.at(1).run(scope.env)?;

    scope.env.insert_global(name, value, false);

    Ok(Trampoline::returning(Expr::Nil))
}

pub fn block(scope: CallScope<'_>) -> Result<Trampoline> {
    scope.assert_at_least(1)?;

    let last = scope.args.last().unwrap();

    for arg in scope.args.iter().take(scope.args.len() - 1).cloned() {
        arg.run(scope.env)?;
    }

    Ok(Trampoline::eval(last.clone()))
}

pub fn setm(scope: CallScope<'_>) -> Result<Trampoline> {
    scope.assert_arity(2)?;

    let name = scope.at(0).assert_identifier()?;
    let value = scope.at(1).run(scope.env)?;

    scope.env.insert_global(name, value, true);

    Ok(Trampoline::returning(Expr::Nil))
}

pub fn if_(scope: CallScope<'_>) -> Result<Trampoline> {
    scope.assert_arity(3)?;

    let condition = scope.at(0).run(scope.env)?.is_true();
    let consequent = scope.at(1);
    let alternative = scope.at(2);

    let expr = if condition { consequent } else { alternative };

    Ok(Trampoline::eval(expr))
}

pub fn less_than(scope: CallScope<'_>) -> Result<Trampoline> {
    scope.assert_arity(2)?;

    let left = scope.at(0).run(scope.env)?.assert_number()?;
    let right = scope.at(1).run(scope.env)?.assert_number()?;

    let value = if left < right {
        Expr::Id("true".to_string())
    } else {
        Expr::Nil
    };

    Ok(Trampoline::returning(value))
}

pub fn expand(scope: CallScope<'_>) -> Result<Trampoline> {
    scope.assert_arity(1)?;

    let value = scope.at(0).run(scope.env)?;
    let value = value.expand(scope.env)?;

    Ok(Trampoline::Return(value))
}

pub fn quote(scope: CallScope<'_>) -> Result<Trampoline> {
    scope.assert_arity(1)?;

    Ok(Trampoline::returning(scope.at(0)))
}

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

pub fn print(scope: CallScope<'_>) -> Result<Trampoline> {
    for arg in scope.args.iter() {
        print!("{}", arg.clone().run(scope.env)?);
    }

    println!();

    Ok(Trampoline::returning(Expr::Nil))
}

pub fn eq(scope: CallScope<'_>) -> Result<Trampoline> {
    scope.assert_arity(2)?;

    let left = scope.at(0).run(scope.env)?;
    let right = scope.at(1).run(scope.env)?;

    let tt = Expr::Id("true".to_string());
    let ff = Expr::Nil;

    let result = match (&left.kind, &right.kind) {
        (Expr::Int(left), Expr::Int(right)) => left == right,
        (Expr::Id(left), Expr::Id(right)) => left == right,
        (Expr::Str(left), Expr::Str(right)) => left == right,
        (Expr::Nil, Expr::Nil) => true,
        _ => false,
    };

    Ok(Trampoline::returning(if result { tt } else { ff }))
}

pub fn head(scope: CallScope<'_>) -> Result<Trampoline> {
    scope.assert_arity(1)?;

    let value = scope.at(0).run(scope.env)?;

    match &value.kind {
        Expr::Cons(head, _) => Ok(Trampoline::Return(head.clone())),
        _ => Err(RuntimeError::ExpectedList(value.to_string())),
    }
}

pub fn tail(scope: CallScope<'_>) -> Result<Trampoline> {
    scope.assert_arity(1)?;

    let value = scope.at(0).run(scope.env)?;

    match &value.kind {
        Expr::Cons(_, tail) => Ok(Trampoline::returning(tail.clone())),
        _ => Err(RuntimeError::ExpectedList(value.to_string())),
    }
}

pub fn cons(scope: CallScope<'_>) -> Result<Trampoline> {
    scope.assert_arity(2)?;

    let head = scope.at(0).run(scope.env)?;
    let tail = scope.at(1).run(scope.env)?;

    Ok(Trampoline::returning(Expr::Cons(head, tail)))
}

pub fn list(scope: CallScope<'_>) -> Result<Trampoline> {
    let mut result = Spanned::new(Expr::Nil, None);

    for arg in scope.args.into_iter().rev() {
        let arg = arg.run(scope.env)?;
        result = Expr::Cons(arg, result.into()).into();
    }

    Ok(Trampoline::returning(result))
}

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

pub fn throw(scope: CallScope<'_>) -> Result<Trampoline> {
    scope.assert_arity(1)?;

    Err(RuntimeError::UserError(scope.at(0).run(scope.env)?))
}

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

pub fn eval(scope: CallScope<'_>) -> Result<Trampoline> {
    scope.assert_arity(1)?;
    let expr = scope.at(0);
    Ok(Trampoline::Eval(expr))
}
