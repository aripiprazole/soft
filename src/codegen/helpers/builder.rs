use llvm_sys::{core::*, prelude::*, LLVMIntPredicate};

use crate::macros::cstr;

pub trait IRBuilder {
    fn position_at_end(&self, block: LLVMBasicBlockRef);
    fn insertion_block(&self) -> LLVMBasicBlockRef;
    fn build_unreachable(&self) -> LLVMValueRef;
    fn build_ret(&self, value: LLVMValueRef) -> LLVMValueRef;
    fn build_load(&self, kind: LLVMTypeRef, value: LLVMValueRef, name: &str) -> LLVMValueRef;
    fn build_store(&self, value: LLVMValueRef, ptr: LLVMValueRef) -> LLVMValueRef;
    fn build_alloca(&self, kind: LLVMTypeRef, name: &str) -> LLVMValueRef;
    fn build_array_alloca(&self, kind: LLVMTypeRef, len: LLVMValueRef, name: &str) -> LLVMValueRef;
    fn build_global_string_ptr(&self, string: &str, name: &str) -> LLVMValueRef;
    fn build_bitcast(&self, value: LLVMValueRef, kind: LLVMTypeRef, name: &str) -> LLVMValueRef;
    fn build_br(&self, block: LLVMBasicBlockRef) -> LLVMValueRef;
    fn build_gep(
        &self,
        kind: LLVMTypeRef,
        value: LLVMValueRef,
        indices: &mut [LLVMValueRef],
        name: &str,
    ) -> LLVMValueRef;
    fn build_cond_br(
        &self,
        cond: LLVMValueRef,
        then: LLVMBasicBlockRef,
        otherwise: LLVMBasicBlockRef,
    ) -> LLVMValueRef;
    fn build_pointer_cast(
        &self,
        value: LLVMValueRef,
        kind: LLVMTypeRef,
        name: &str,
    ) -> LLVMValueRef;
    fn build_icmp(
        &self,
        pred: LLVMIntPredicate,
        lhs: LLVMValueRef,
        rhs: LLVMValueRef,
        name: &str,
    ) -> LLVMValueRef;
    fn build_call(
        &self,
        kind: LLVMTypeRef,
        function: LLVMValueRef,
        args: &mut [LLVMValueRef],
        name: &str,
    ) -> LLVMValueRef;
}

impl IRBuilder for LLVMBuilderRef {
    fn insertion_block(&self) -> LLVMBasicBlockRef {
        unsafe { LLVMGetInsertBlock(*self) }
    }

    fn position_at_end(&self, block: LLVMBasicBlockRef) {
        unsafe { LLVMPositionBuilderAtEnd(*self, block) }
    }

    fn build_unreachable(&self) -> LLVMValueRef {
        unsafe { LLVMBuildUnreachable(*self) }
    }

    fn build_ret(&self, value: LLVMValueRef) -> LLVMValueRef {
        unsafe { LLVMBuildRet(*self, value) }
    }

    fn build_load(&self, kind: LLVMTypeRef, value: LLVMValueRef, name: &str) -> LLVMValueRef {
        unsafe { LLVMBuildLoad2(*self, kind, value, cstr!(name)) }
    }

    fn build_store(&self, value: LLVMValueRef, ptr: LLVMValueRef) -> LLVMValueRef {
        unsafe { LLVMBuildStore(*self, value, ptr) }
    }

    fn build_alloca(&self, kind: LLVMTypeRef, name: &str) -> LLVMValueRef {
        unsafe { LLVMBuildAlloca(*self, kind, cstr!(name)) }
    }

    fn build_array_alloca(&self, kind: LLVMTypeRef, len: LLVMValueRef, name: &str) -> LLVMValueRef {
        unsafe { LLVMBuildArrayAlloca(*self, kind, len, cstr!(name)) }
    }

    fn build_global_string_ptr(&self, string: &str, name: &str) -> LLVMValueRef {
        unsafe { LLVMBuildGlobalStringPtr(*self, cstr!(string), cstr!(name)) }
    }

    fn build_bitcast(&self, value: LLVMValueRef, kind: LLVMTypeRef, name: &str) -> LLVMValueRef {
        unsafe { LLVMBuildBitCast(*self, value, kind, cstr!(name)) }
    }

    fn build_br(&self, block: LLVMBasicBlockRef) -> LLVMValueRef {
        unsafe { LLVMBuildBr(*self, block) }
    }

    fn build_gep(
        &self,
        kind: LLVMTypeRef,
        value: LLVMValueRef,
        indices: &mut [LLVMValueRef],
        name: &str,
    ) -> LLVMValueRef {
        unsafe {
            let len = indices.len();
            LLVMBuildGEP2(
                *self,
                kind,
                value,
                indices.as_mut_ptr(),
                len as u32,
                cstr!(name),
            )
        }
    }

    fn build_cond_br(
        &self,
        cond: LLVMValueRef,
        then: LLVMBasicBlockRef,
        otherwise: LLVMBasicBlockRef,
    ) -> LLVMValueRef {
        unsafe { LLVMBuildCondBr(*self, cond, then, otherwise) }
    }

    fn build_pointer_cast(
        &self,
        value: LLVMValueRef,
        kind: LLVMTypeRef,
        name: &str,
    ) -> LLVMValueRef {
        unsafe { LLVMBuildPointerCast(*self, value, kind, cstr!(name)) }
    }

    fn build_call(
        &self,
        kind: LLVMTypeRef,
        function: LLVMValueRef,
        args: &mut [LLVMValueRef],
        name: &str,
    ) -> LLVMValueRef {
        unsafe {
            let len = args.len();
            LLVMBuildCall2(
                *self,
                kind,
                function,
                args.as_mut_ptr(),
                len as u32,
                cstr!(name),
            )
        }
    }

    fn build_icmp(
        &self,
        pred: LLVMIntPredicate,
        lhs: LLVMValueRef,
        rhs: LLVMValueRef,
        name: &str,
    ) -> LLVMValueRef {
        unsafe { LLVMBuildICmp(*self, pred, lhs, rhs, cstr!(name)) }
    }
}
