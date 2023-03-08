pub use crate::runtime::{Value, ValueRef};
use std::alloc::{alloc, Layout};

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
        let layout = Layout::from_size_align(std::mem::size_of::<ValueRef>() * env_len as usize, 8)
            .expect("Invalid layout");
        let slice = alloc(layout) as *mut ValueRef;
        *slice = std::mem::copy(env.as_ref().unwrap());
        let env = ValueRef::vec(env_len as usize, slice);
        ValueRef::closure(env, body)
    }

    #[no_mangle]
    pub unsafe extern "C" fn prim__Value_function(
        arity: u64,
        function_ptr: *mut libc::c_void,
    ) -> ValueRef {
        ValueRef::function(arity as _, function_ptr)
    }

    #[no_mangle]
    pub unsafe extern "C" fn prim__Value_gep(ptr: ValueRef, index: u64) -> ValueRef {
        if ptr.is_num() {
            panic!("prim__Value_gep: expected pointer, got number");
        }

        match ptr.to_value() {
            Value::Vec(ref items, _) => items.add(index as _).read(),
            _ => panic!("prim__Value_gep: expected pointer, got {ptr:?}"),
        }
    }
}

pub mod fun {
    use super::{Value, ValueRef};

    #[no_mangle]
    pub unsafe extern "C" fn prim__fn_addr(value: ValueRef) -> *mut libc::c_void {
        if value.is_num() {
            panic!("prim__fn_addr: expected pointer, got number");
        }

        match value.to_value() {
            Value::Function(_, ptr) => *ptr,
            _ => panic!("prim__fn_addr: expected pointer, got {value:?}"),
        }
    }

    #[no_mangle]
    pub unsafe extern "C" fn prim__check_arity(value: ValueRef, args: u64) -> *mut libc::c_void {
        if value.is_num() {
            panic!("prim__fn_check_arity: expected pointer, got number");
        }

        match value.to_value() {
            Value::Function(arity, ptr) => {
                if (*arity as u64) != args {
                    panic!("prim__fn_check_arity: expected arity {value}, got {args}");
                }

                *ptr
            }
            _ => std::ptr::null_mut(),
        }
    }
}

pub mod closure {
    use super::{Value, ValueRef};

    #[no_mangle]
    pub unsafe extern "C" fn prim__closure_get_env(value: ValueRef) -> *mut libc::c_void {
        if value.is_num() {
            panic!("prim__closure_get_env: expected pointer, got number");
        }

        match value.to_value() {
            Value::Closure(env, _) => std::mem::transmute::<u64, *mut libc::c_void>(env.0),
            _ => std::ptr::null_mut(),
        }
    }

    #[no_mangle]
    pub unsafe extern "C" fn prim__closure_get_fn(value: ValueRef) -> *mut libc::c_void {
        if value.is_num() {
            panic!("prim__closure_get_fn: expected pointer, got number");
        }

        match value.to_value() {
            Value::Closure(_, value) => std::mem::transmute::<u64, *mut libc::c_void>(value.0),
            _ => std::ptr::null_mut(),
        }
    }
}

pub mod global {
    use std::ffi::CStr;

    use crate::codegen::jit::{GlobalEnvironment, GlobalRef};

    use super::ValueRef;

    #[no_mangle]
    pub unsafe extern "C" fn prim__is_null(value: *mut libc::c_void) -> bool {
        value.is_null()
    }

    #[no_mangle]
    pub unsafe extern "C" fn prim__global_get(
        global_environment: *mut GlobalEnvironment,
        name: *const i8,
    ) -> ValueRef {
        let rust_name = CStr::from_ptr(name).to_string_lossy();
        let global_ref = global_environment
            .as_ref()
            .unwrap()
            .symbols
            .get(rust_name.as_ref())
            .expect(&format!("prim__global_get: symbol {rust_name} not found"));

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

    #[no_mangle]
    pub unsafe extern "C" fn prim__panic() -> ValueRef {
        panic!("prim__panic: panic!");
    }
}
