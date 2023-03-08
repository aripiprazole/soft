use super::*;

pub mod builder;
pub mod context;
pub mod module;
pub mod types;

pub use builder::*;
pub use context::*;
pub use module::*;
pub use types::*;

impl Codegen {
    pub fn make_int_const(&self, value: u64) -> LLVMValueRef {
        unsafe { LLVMConstInt(self.types.u64, value, 0) }
    }

    pub fn true_value(&self) -> LLVMValueRef {
        self.make_int_const(1)
    }

    pub fn false_value(&self) -> LLVMValueRef {
        self.make_int_const(1)
    }
}
