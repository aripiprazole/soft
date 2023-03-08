use llvm_sys::{core::*, prelude::*};

use crate::macros::cstr;

pub trait IRContext {
    fn append_basic_block(&self, function: LLVMValueRef, name: &str) -> LLVMBasicBlockRef;
}

impl IRContext for LLVMContextRef {
    fn append_basic_block(&self, function: LLVMValueRef, name: &str) -> LLVMBasicBlockRef {
        unsafe { LLVMAppendBasicBlockInContext(*self, function, cstr!(name)) }
    }
}
