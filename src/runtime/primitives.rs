use crate::codegen::Codegen;
pub use crate::runtime::{Value, ValueRef};
use std::alloc::{alloc, Layout};

pub type AnyPtr = *mut libc::c_void;

pub mod value {
    use std::mem::size_of;

    pub use super::*;

    #[no_mangle]
    pub extern "C" fn prim__Value_new_num(n: u64) -> ValueRef {
        ValueRef::new_num(n)
    }

    #[no_mangle]
    pub extern "C" fn prim__Value_cons(head: ValueRef, tail: ValueRef) -> ValueRef {
        ValueRef::cons(head, tail)
    }

    #[no_mangle]
    pub extern "C" fn prim__Value_nil() -> ValueRef {
        ValueRef::nil()
    }

    #[no_mangle]
    pub extern "C" fn prim__Value_head(list: ValueRef) -> ValueRef {
        if list.is_num() {
            panic!("prim__Value_head: expected list, got number");
        }

        match list.to_value() {
            Value::Cons(head, _) => *head,
            _ => panic!("prim__Value_head: expected list, got {:?}", list),
        }
    }

    #[no_mangle]
    pub extern "C" fn prim__Value_tail(list: ValueRef) -> ValueRef {
        if list.is_num() {
            panic!("prim__Value_tail: expected list, got number");
        }

        match list.to_value() {
            Value::Cons(_, tail) => *tail,
            _ => panic!("prim__Value_tail: expected list, got {:?}", list),
        }
    }

    #[no_mangle]
    pub extern "C" fn prim__Value_is_true(value: ValueRef) -> bool {
        if value.is_num() {
            value.num() != 0
        } else {
            !matches!(value.to_value(), Value::Nil)
        }
    }

    #[no_mangle]
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    pub extern "C" fn prim__Value_new_closure(
        env: *mut ValueRef,
        env_len: u64,
        body: ValueRef,
    ) -> ValueRef {
        unsafe {
            let layout = Layout::from_size_align(size_of::<ValueRef>() * env_len as usize, 8)
                .expect("Invalid layout");

            let slice = alloc(layout) as *mut ValueRef;
            *slice = std::mem::copy(env.as_ref().unwrap());

            let env = ValueRef::vec(env_len as usize, slice);
            ValueRef::closure(env, body)
        }
    }

    #[no_mangle]
    pub extern "C" fn prim__Value_function(arity: u64, function_ptr: AnyPtr) -> ValueRef {
        ValueRef::function(arity as _, function_ptr)
    }

    #[no_mangle]
    pub extern "C" fn prim__Value_gep(ptr: ValueRef, index: u64) -> ValueRef {
        if ptr.is_num() {
            panic!("prim__Value_gep: expected pointer, got number");
        }

        match ptr.to_value() {
            Value::Vec(ref items, _) => unsafe { items.add(index as _).read() },
            _ => panic!("prim__Value_gep: expected pointer, got {ptr:?}"),
        }
    }
}

pub mod fun {
    use super::*;

    #[no_mangle]
    pub extern "C" fn prim__fn_addr(value: ValueRef) -> AnyPtr {
        if value.is_num() {
            panic!("prim__fn_addr: expected pointer, got number");
        }

        match value.to_value() {
            Value::Function(_, ptr) => *ptr,
            _ => panic!("prim__fn_addr: expected pointer, got {value:?}"),
        }
    }

    #[no_mangle]
    pub extern "C" fn prim__check_arity(value: ValueRef, args: u64) -> AnyPtr {
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
}

pub mod global {
    use std::ffi::CStr;

    use crate::codegen::jit::{GlobalEnvironment, GlobalRef};

    use super::*;

    #[no_mangle]
    pub extern "C" fn prim__is_null(value: AnyPtr) -> bool {
        value.is_null()
    }

    #[no_mangle]
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    pub extern "C" fn prim__global_get(
        global: *mut GlobalEnvironment,
        name: *const i8,
    ) -> ValueRef {
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
}

macro_rules! f {
    ($n:ident) => {
        (stringify!($n), $n as *mut libc::c_void)
    };
}

impl Codegen {
    pub fn install_primitives(mut self) -> Self {
        use crate::runtime::primitives::closure::*;
        use crate::runtime::primitives::fun::*;
        use crate::runtime::primitives::global::*;
        use crate::runtime::primitives::value::*;

        let types = &self.types;
        let ctx = &mut self.environment;

        ctx.with(f!(prim__Value_new_num), types.ptr, [types.u64]);
        ctx.with(f!(prim__Value_cons), types.ptr, [types.ptr, types.ptr]);
        ctx.with(f!(prim__Value_nil), types.ptr, []);
        ctx.with(f!(prim__Value_is_true), types.i1, [types.ptr]);
        ctx.with(f!(prim__Value_function), types.ptr, [types.u64, types.ptr]);
        ctx.with(f!(prim__Value_gep), types.ptr, [types.ptr, types.u64]);
        ctx.with(
            f!(prim__Value_new_closure),
            types.ptr,
            [types.ptr, types.u64, types.ptr],
        );

        ctx.with(f!(prim__global_get), types.ptr, [types.ptr, types.ptr]);
        ctx.with(f!(prim__global_set), types.ptr, [types.ptr; 3]);

        ctx.with(f!(prim__fn_addr), types.ptr, [types.ptr]);
        ctx.with(f!(prim__check_arity), types.ptr, [types.ptr, types.u64]);
        ctx.with(f!(prim__closure_get_env), types.ptr, [types.ptr]);
        ctx.with(f!(prim__closure_get_fn), types.ptr, [types.ptr]);
        ctx.with(f!(prim__is_null), types.i1, [types.ptr]);
        ctx.with(f!(prim__panic), types.ptr, []);

        self
    }
}
