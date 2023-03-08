use llvm_sys::LLVMIntPredicate;

use super::*;

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

    fn build_icmp(
        &self,
        pred: LLVMIntPredicate,
        lhs: LLVMValueRef,
        rhs: LLVMValueRef,
        name: &str,
    ) -> LLVMValueRef {
        unsafe { LLVMBuildICmp(*self, pred, lhs, rhs, cstr!(name)) }
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
}

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

pub trait IRContext {
    fn append_basic_block(&self, function: LLVMValueRef, name: &str) -> LLVMBasicBlockRef;
}

impl IRContext for LLVMContextRef {
    fn append_basic_block(&self, function: LLVMValueRef, name: &str) -> LLVMBasicBlockRef {
        unsafe { LLVMAppendBasicBlockInContext(*self, function, cstr!(name)) }
    }
}

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

macro_rules! pointer_type {
    ($e:expr) => {
        unsafe { LLVMPointerType($e, 0) }
    };
}

macro_rules! function_type {
    ($return_type:expr, $arg_type:expr; $n:expr) => {{
        let mut args_types = vec![$arg_type; $n];
        unsafe {
            LLVMFunctionType($return_type, args_types.as_mut_ptr(), args_types.len() as u32, 0)
        }
    }};
    ($return_type:expr, $($args:expr),+ $(,)?) => {{
        let mut args_types = [$($args),+];
        unsafe {
            LLVMFunctionType($return_type, args_types.as_mut_ptr(), args_types.len() as u32, 0)
        }
    }};
}

pub(crate) use function_type;
pub(crate) use pointer_type;
