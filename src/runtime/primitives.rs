pub use crate::runtime::{Value, ValueRef};

pub mod value {
    pub use super::*;

    #[no_mangle]
    pub unsafe extern "C" fn prim__Value_new_num(n: u64) -> ValueRef {
        ValueRef::new_num(n)
    }

    #[no_mangle]
    pub unsafe extern "C" fn prim__Value_cons(head: ValueRef, tail: ValueRef) -> ValueRef {
        ValueRef::cons(head, tail)
    }

    #[no_mangle]
    pub unsafe extern "C" fn prim__Value_nil() -> ValueRef {
        ValueRef::nil()
    }

    #[no_mangle]
    pub unsafe extern "C" fn prim__Value_head(list: ValueRef) -> ValueRef {
        if list.is_num() {
            panic!("prim__Value_head: expected list, got number");
        }

        match list.to_value() {
            Value::Cons(head, _) => *head,
            _ => panic!("prim__Value_head: expected list, got {:?}", list),
        }
    }

    #[no_mangle]
    pub unsafe extern "C" fn prim__Value_tail(list: ValueRef) -> ValueRef {
        if list.is_num() {
            panic!("prim__Value_tail: expected list, got number");
        }

        match list.to_value() {
            Value::Cons(_, tail) => *tail,
            _ => panic!("prim__Value_tail: expected list, got {:?}", list),
        }
    }

    #[no_mangle]
    pub unsafe extern "C" fn prim__Value_is_true(value: ValueRef) -> bool {
        if value.is_num() {
            value.num() != 0
        } else {
            !matches!(value.to_value(), Value::Nil)
        }
    }

    #[no_mangle]
    pub unsafe extern "C" fn prim__Value_new_closure(
        env: *mut ValueRef,
        env_len: u64,
        body: ValueRef,
    ) -> ValueRef {
        let env = ValueRef::vec(std::slice::from_raw_parts(env, env_len as _).into());

        ValueRef::closure(env, body)
    }

    #[no_mangle]
    pub unsafe extern "C" fn prim__Value_function(
        arity: u64,
        function_ptr: *mut libc::c_void,
    ) -> ValueRef {
        ValueRef::function(arity as _, function_ptr)
    }
}

pub mod global {
    use std::ffi::CStr;

    use crate::codegen::jit::{GlobalEnvironment, GlobalRef};

    use super::ValueRef;

    #[no_mangle]
    pub unsafe extern "C" fn prim__global_get(
        global_environment: *mut GlobalEnvironment,
        name: *const i8,
    ) -> ValueRef {
        let global_ref = global_environment
            .as_ref()
            .unwrap()
            .symbols
            .get(CStr::from_ptr(name).to_string_lossy().as_ref())
            .expect("prim__global_environment_get: symbol not found");

        global_ref.addr
    }

    #[no_mangle]
    pub unsafe extern "C" fn prim__global_set(
        global_environment: *mut GlobalEnvironment,
        name: *const i8,
        value: ValueRef,
    ) {
        let name = CStr::from_ptr(name).to_string_lossy().into_owned();
        let global_ref = GlobalRef::new(value);

        global_environment
            .as_mut()
            .unwrap()
            .symbols
            .insert(name, global_ref);
    }
}
