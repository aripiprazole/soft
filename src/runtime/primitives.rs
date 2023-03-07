use crate::codegen::Codegen;
pub use crate::runtime::{Value, ValueRef};
use std::alloc::{alloc, Layout};

pub type AnyPtr = *mut libc::c_void;

pub mod closure;
pub mod fun;
pub mod global;
pub mod value;

macro_rules! f {
    ($n:ident) => {
        (stringify!($n), $n as AnyPtr)
    };
}

impl Codegen {
    pub fn install_primitives(mut self) -> Self {
        let types = &self.types;
        let ctx = &mut self.environment;
        use closure::*;
        use fun::*;
        use global::*;
        use value::*;

        ctx.with(f!(prim__Value_new_num), types.ptr, [types.u64]);
        ctx.with(f!(prim__Value_cons), types.ptr, [types.ptr, types.ptr]);
        ctx.with(f!(prim__Value_nil), types.ptr, []);
        ctx.with(f!(prim__Value_is_true), types.i1, [types.ptr]);
        ctx.with(f!(prim__Value_function), types.ptr, [types.u64, types.ptr]);
        ctx.with(f!(prim__Value_gep), types.ptr, [types.ptr, types.u64]);

        ctx.with(f!(prim__closure_get_env), types.ptr, [types.ptr]);
        ctx.with(f!(prim__closure_get_fn), types.ptr, [types.ptr]);
        ctx.with(
            f!(prim__closure),
            types.ptr,
            [types.ptr, types.u64, types.ptr],
        );

        ctx.with(f!(prim__global_get), types.ptr, [types.ptr, types.ptr]);
        ctx.with(f!(prim__global_set), types.ptr, [types.ptr; 3]);
        ctx.with(f!(prim__is_null), types.i1, [types.ptr]);
        ctx.with(f!(prim__panic), types.ptr, []);

        ctx.with(f!(prim__fn_addr), types.ptr, [types.ptr]);
        ctx.with(f!(prim__check_arity), types.ptr, [types.ptr, types.u64]);

        self
    }
}
