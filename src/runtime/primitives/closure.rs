use std::mem::size_of;

use super::*;

#[no_mangle]
pub extern "C" fn prim__closure_get_env(value: ValueRef) -> AnyPtr {
    if value.is_num() {
        panic!("prim__closure_get_env: expected pointer, got number");
    }

    match value.to_value() {
        Value::Closure(env, _) => env.0 as AnyPtr,
        _ => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "C" fn prim__closure_get_fn(value: ValueRef) -> AnyPtr {
    if value.is_num() {
        panic!("prim__closure_get_fn: expected pointer, got number");
    }

    match value.to_value() {
        Value::Closure(_, value) => value.0 as AnyPtr,
        _ => std::ptr::null_mut(),
    }
}

#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn prim__closure(env: *mut ValueRef, env_len: u64, body: ValueRef) -> ValueRef {
    unsafe {
        let layout = Layout::from_size_align(size_of::<ValueRef>() * env_len as usize, 8)
            .expect("Invalid layout");

        let slice = alloc(layout) as *mut ValueRef;
        *slice = std::mem::copy(env.as_ref().unwrap());

        let env = ValueRef::vec(env_len as usize, slice);
        ValueRef::closure(env, body)
    }
}
