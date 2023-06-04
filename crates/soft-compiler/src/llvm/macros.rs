#![allow(unused_imports)]
#![allow(unused_macros)]

macro_rules! std_llvm_type {
    ($codegen:expr, void) => {
        $codegen.ctx.void_type()
    };
    ($codegen:expr, ctx) => {
        $codegen
            .ctx
            .i8_type()
            .ptr_type(inkwell::AddressSpace::default())
    };
    ($codegen:expr, bool) => {
        $codegen.ctx.bool_type()
    };
    ($codegen:expr, u8) => {
        $codegen.ctx.i8_type()
    };
    ($codegen:expr, u64) => {
        $codegen.ctx.i64_type()
    };
    ($codegen:expr, $e:expr) => {
        $e
    };
}

macro_rules! build_std_functions {
    ($codegen:expr, {$($name:ident($($x:tt),* $(,)?) -> $ret:tt),+ $(,)?}) => {{
        $({
            if $codegen.module.get_function(stringify!($name)).is_none() {
                let name = stringify!($name);
                let ret = $crate::llvm::macros::std_llvm_type!($codegen, $ret);
                let args = &[$($crate::llvm::macros::std_llvm_type!($codegen, $x).into()),*];
                $codegen.module.add_function(name, ret.fn_type(args, false), None);
            }
        })+
    }};
}

macro_rules! register_jit_function {
    ($codegen:expr, $engine:expr, $name:expr) => {{
        let fun = $codegen.module.get_function(stringify!($name)).unwrap();
        $engine.add_global_mapping(&fun, $name as usize);
    }};
}

macro_rules! std_function {
    ($name:ident(ctx) -> u64) => {
        #[allow(clippy::needless_lifetimes)]
        #[allow(non_snake_case)]
        pub fn $name<'b>(&'b self) -> inkwell::values::BasicValueEnum<'b> {
            self.call_std(stringify!($name), &[])
        }
    };
    ($name:ident(ctx, $($argsn:ident), + $(,)?)) => {
        #[allow(clippy::needless_lifetimes)]
        #[allow(non_snake_case)]
        pub fn $name<'b>(&'b self, $($argsn: inkwell::values::BasicValueEnum<'b>),+) -> inkwell::values::BasicValueEnum<'b> {
            let arguments = &[$($argsn.into()),+];
            self.call_std(stringify!($name), arguments)
        }
    };
    ($name:ident($($argsn:ident), * $(,)?)) => {
        #[allow(clippy::needless_lifetimes)]
        #[allow(non_snake_case)]
        pub fn $name<'b>(&'b self, $($argsn: inkwell::values::BasicValueEnum<'b>),*) -> inkwell::values::BasicValueEnum<'b> {
            let arguments = &[$($argsn.into()),*];
            self.call(stringify!($name), arguments)
        }
    };
}

pub(crate) use build_std_functions;
pub(crate) use register_jit_function;
pub(crate) use std_function;
pub(crate) use std_llvm_type;
