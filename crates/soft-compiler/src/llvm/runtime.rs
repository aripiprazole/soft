use super::macros;
use super::Codegen;

impl<'guard> Codegen<'guard> {
    /// Initialize the `internal` and the `standard-library` functions that are used by the compiler
    /// for compilation purposes.
    ///
    /// It does uses the macro [macros::build_std_functions], that passes a list of functions and
    /// registers into the LLVM context.
    pub fn initialize_std_functions(&self) {
        let object = self.context.i64_type();

        macros::build_std_functions!(self, {
            new_u61_object(u64) -> object
        });
    }
}
