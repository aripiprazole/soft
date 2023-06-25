use std::ffi::{CStr, CString};

use libc::RTLD_LAZY;

use crate::{
    error::{Result, RuntimeError},
    value::{CallScope, Expr, Trampoline, Type},
};

pub fn ffi_open(scope: CallScope<'_>) -> Result<Trampoline> {
    scope.assert_arity(1)?;

    let path = scope.at(0).run(scope.env)?.assert_string()?;

    let str = CString::new(path).unwrap();
    let lib = unsafe { libc::dlopen(str.as_ptr(), RTLD_LAZY) };

    if lib.is_null() {
        Err(RuntimeError::from("failed to open library"))
    } else {
        Ok(Trampoline::returning(Expr::Library(lib)))
    }
}

pub fn ffi_get(scope: CallScope<'_>) -> Result<Trampoline> {
    scope.assert_arity(3)?;

    let lib = scope.at(0).run(scope.env)?.assert_library()?;
    let symbol = scope.at(1).run(scope.env)?.assert_string()?;
    let types = scope.at(2).assert_list()?;

    let str = CString::new(symbol).unwrap();
    let symbol = unsafe { libc::dlsym(lib, str.as_ptr()) };

    let mut type_list = Vec::new();

    for typ in types {
        match typ.assert_identifier()?.as_str() {
            "int" => type_list.push(Type::Int),
            "string" => type_list.push(Type::String),
            _ => return Err(RuntimeError::from("invalid type")),
        }
    }

    if symbol.is_null() {
        Err(RuntimeError::from("failed to get symbol"))
    } else {
        Ok(Trampoline::returning(Expr::External(symbol, type_list)))
    }
}

pub fn ffi_apply(scope: CallScope<'_>) -> Result<Trampoline> {
    scope.assert_arity(2)?;

    let (func, types) = scope.at(0).run(scope.env)?.assert_external()?;
    let args = scope.at(1).run(scope.env)?.assert_list()?;

    let mut arg_list = Vec::new();
    let mut c_strings = Vec::new();

    if args.len() != types.len() - 1 {
        return Err(RuntimeError::from("invalid number of arguments"));
    }

    let ret_type = types.last().cloned().unwrap();

    for (arg, typ) in args.into_iter().zip(types) {
        match typ {
            Type::Int => arg_list.push(arg.assert_number()?),
            Type::String => {
                let str = arg.assert_string()?;
                let c_str = CString::new(str).unwrap();
                arg_list.push(c_str.as_ptr() as i64);
                c_strings.push(c_str);
            }
        }
    }

    let result = unsafe {
        let func: extern "C" fn(*const i64) -> i64 = std::mem::transmute(func);
        func(arg_list.as_ptr())
    };

    match ret_type {
        Type::Int => Ok(Trampoline::returning(Expr::Int(result))),
        Type::String => {
            let c_str = unsafe { CStr::from_ptr(result as *mut i8) };
            let str = c_str.to_str().unwrap().to_owned();

            Ok(Trampoline::returning(Expr::Str(str)))
        }
    }
}
