use llvm_sys::{core::*, prelude::*};

use crate::macros::cstr;

pub trait IRModule {
    fn add_function(
        &self,
        name: &str,
        args: &mut [LLVMTypeRef],
        return_type: LLVMTypeRef,
    ) -> LLVMValueRef;
}

impl IRModule for LLVMModuleRef {
    fn add_function(
        &self,
        name: &str,
        args: &mut [LLVMTypeRef],
        return_type: LLVMTypeRef,
    ) -> LLVMValueRef {
        unsafe {
            let len = args.len();
            let function_type = LLVMFunctionType(return_type, args.as_mut_ptr(), len as u32, 0);
            LLVMAddFunction(*self, cstr!(name), function_type)
        }
    }
}
