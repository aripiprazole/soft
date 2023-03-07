
use std::ffi::CStr;

use crate::codegen::jit::{GlobalEnvironment, GlobalRef};

use super::*;

#[no_mangle]
pub extern "C" fn prim__is_null(value: AnyPtr) -> bool {
    value.is_null()
}

#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn prim__global_get(global: *mut GlobalEnvironment, name: *const i8) -> ValueRef {
    unsafe {
        let rust_name = CStr::from_ptr(name).to_string_lossy();
        let global_ref = global
            .as_ref()
            .unwrap()
            .symbols
            .get(rust_name.as_ref())
            .unwrap_or_else(|| panic!("prim__global_get: symbol {rust_name} not found"));

        global_ref.addr
    }
}

#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn prim__global_set(
    global: *mut GlobalEnvironment,
    name: *const i8,
    value: ValueRef,
) {
    unsafe {
        let name = CStr::from_ptr(name).to_string_lossy().into_owned();
        let global_ref = GlobalRef::new(value);

        global.as_mut().unwrap().symbols.insert(name, global_ref);
    }
}

#[no_mangle]
pub extern "C" fn prim__panic() -> ValueRef {
    panic!("prim__panic: panic!");
}
