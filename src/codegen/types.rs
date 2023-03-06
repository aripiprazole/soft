use super::*;

llvm_wrapper!(Type, LLVMTypeRef, LLVMPrintTypeToString);
llvm_wrapper!(Value, LLVMValueRef, LLVMPrintValueToString);

pub struct Types {
    pub void_ptr: LLVMTypeRef,
    pub u64: LLVMTypeRef,
}

impl Types {
    pub unsafe fn try_new(context: LLVMContextRef) -> Result<Self, CodegenError> {
        Ok(Self {
            void_ptr: LLVMVoidTypeInContext(context),
            u64: LLVMInt64TypeInContext(context),
        })
    }
}
