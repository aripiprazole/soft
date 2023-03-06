use super::*;

llvm_wrapper!(Type, LLVMTypeRef, LLVMPrintTypeToString);
llvm_wrapper!(Value, LLVMValueRef, LLVMPrintValueToString);

pub struct Types {
    pub ptr: LLVMTypeRef,
    pub u64: LLVMTypeRef,
}

impl Types {
    pub unsafe fn try_new(context: LLVMContextRef) -> Result<Self, CodegenError> {
        Ok(Self {
            ptr: LLVMPointerType(LLVMInt8TypeInContext(context), 0),
            u64: LLVMInt64TypeInContext(context),
        })
    }
}
