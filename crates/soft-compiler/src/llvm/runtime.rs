use inkwell::values::BasicMetadataValueEnum;
use inkwell::values::BasicValueEnum;

use super::macros;
use super::macros::std_function;
use super::Codegen;

impl<'guard> Codegen<'guard> {
    /// Initialize the `internal` and the `standard-library` functions that are used by the compiler
    /// for compilation purposes.
    ///
    /// It does uses the macro [macros::build_std_functions], that passes a list of functions and
    /// registers into the LLVM context.
    pub fn initialize_std_functions(&self) {
        let object = self.ctx.i64_type();

        macros::build_std_functions!(self, {
            new_u61_object(u64) -> object
        });
    }

    std_function!(new_u61_object(value));

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
    pub fn call<'a>(
        &'a self,
        name: &str,
        args: &[BasicMetadataValueEnum<'a>],
    ) -> BasicValueEnum<'a> {
        self.builder
            .build_direct_call(self.module.get_function(name).unwrap(), args.as_ref(), "")
            .try_as_basic_value()
            .left()
            .unwrap_or_else(|| panic!("{} should return a BasicValueEnum", name))
    }
}
