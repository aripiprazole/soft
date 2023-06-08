use inkwell::attributes::Attribute;
use inkwell::attributes::AttributeLoc;
use inkwell::execution_engine::ExecutionEngine;
use inkwell::values::BasicMetadataValueEnum;
use inkwell::values::BasicValueEnum;

use super::macros;
use super::macros::std_function;
use super::Codegen;

use soft_runtime::internal::*;

impl<'guard> Codegen<'guard> {
    /// Initialize the `internal` and the `standard-library` functions that are used by the compiler
    /// for compilation purposes.
    ///
    /// It does uses the macro [macros::build_std_functions], that passes a list of functions and
    /// registers into the LLVM context.
    pub fn initialize_std_functions(&self) {
        macros::build_std_functions!(self, {
            prim__new_u61(u64) -> u64,
            prim__function(ptr) -> u64,
            prim__add_tagged(u64, u64) -> u64,
            prim__sub_tagged(u64, u64) -> u64,
            prim__mul_tagged(u64, u64) -> u64,
            prim__mod_tagged(u64, u64) -> u64,
            prim__shl_tagged(u64, u64) -> u64,
            prim__shr_tagged(u64, u64) -> u64,
            prim__and_tagged(u64, u64) -> u64,
            prim__xor_tagged(u64, u64) -> u64,
            prim__or_tagged(u64, u64) -> u64,
            prim__nil() -> u64,
            soft_panic(str) -> u64,
        });
    }

    pub fn initialize_jit_functions(&self, engine: &ExecutionEngine) {
        macros::register_jit_function!(
            self,
            engine,
            [
                prim__new_u61,
                prim__function,
                prim__add_tagged,
                prim__sub_tagged,
                prim__mul_tagged,
                prim__mod_tagged,
                prim__shl_tagged,
                prim__shr_tagged,
                prim__and_tagged,
                prim__xor_tagged,
                prim__or_tagged,
                prim__nil,
                soft_panic,
            ]
        );
    }

    pub fn setup_attributes(&self) {
        let noreturn = self.attr("noreturn");
        let uwtable = self.attr("uwtable");
        let noinline = self.attr("noinline");

        {
            let f = self.module.get_function(stringify!(soft_panic)).unwrap();
            f.add_attribute(AttributeLoc::Function, noinline);
            f.add_attribute(AttributeLoc::Function, noreturn);
            f.add_attribute(AttributeLoc::Function, uwtable);
        }
    }

    std_function!(prim__new_u61(value));
    std_function!(prim__function(value));
    std_function!(prim__add_tagged(lhs, rhs));
    std_function!(prim__sub_tagged(lhs, rhs));
    std_function!(prim__mul_tagged(lhs, rhs));
    std_function!(prim__mod_tagged(lhs, rhs));
    std_function!(prim__shl_tagged(lhs, rhs));
    std_function!(prim__shr_tagged(lhs, rhs));
    std_function!(prim__and_tagged(lhs, rhs));
    std_function!(prim__xor_tagged(lhs, rhs));
    std_function!(prim__or_tagged(lhs, rhs));
    std_function!(prim__nil());
    std_function!(soft_panic(message));

    pub fn attr(&self, name: &str) -> Attribute {
        let attr = Attribute::get_named_enum_kind_id(name);
        self.ctx.create_enum_attribute(attr, 1)
    }

    /// Call a function from the Soft runtime, that passes the context as the first argument.
    /// This is used for functions that are not part of the MIR, but are part of the runtime.
    ///
    /// # Example
    /// ```rust (ignore)
    /// let term = self.build_term(*term);
    ///
    /// self.call_std("some_fn", &[term.into()])
    /// ```
    pub fn call_std<'a>(
        &'a self,
        name: &str,
        args: &[BasicMetadataValueEnum<'a>],
    ) -> BasicValueEnum<'a> {
        let mut complete_args: Vec<BasicMetadataValueEnum> = vec![];
        complete_args.extend_from_slice(args);

        self.builder
            .build_direct_call(
                self.module
                    .get_function(name)
                    .unwrap_or_else(|| panic!("Function {name} not found in module")),
                complete_args.as_ref(),
                "",
            )
            .try_as_basic_value()
            .left()
            .unwrap_or_else(|| panic!("{} should return a BasicValueEnum", name))
    }

    /// Call a function a function that returns a [BasicValueEnum].
    pub fn call(
        &self,
        name: &str,
        args: &[BasicMetadataValueEnum<'guard>],
    ) -> BasicValueEnum<'guard> {
        self.builder
            .build_direct_call(self.module.get_function(name).unwrap(), args.as_ref(), "")
            .try_as_basic_value()
            .left()
            .unwrap_or_else(|| panic!("{} should return a BasicValueEnum", name))
    }
}
