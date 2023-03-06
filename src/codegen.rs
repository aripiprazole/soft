use std::ffi::{c_void, CStr};

use llvm_sys::{
    analysis::{LLVMVerifierFailureAction::LLVMReturnStatusAction, LLVMVerifyModule},
    error::LLVMDisposeErrorMessage,
    target::{LLVM_InitializeNativeAsmPrinter, LLVM_InitializeNativeTarget},
    LLVMDiagnosticSeverity::{LLVMDSError, LLVMDSNote, LLVMDSRemark, LLVMDSWarning},
};

use crate::util::{cstr, llvm_wrapper};

pub use llvm_sys::{core::*, execution_engine::*, prelude::*};

pub mod compile;
pub mod execution;
pub mod types;

pub type CodegenError = String;

pub struct Codegen {
    pub context: LLVMContextRef,
    pub module: LLVMModuleRef,
    pub builder: LLVMBuilderRef,
    pub types: types::Types,
}

impl Drop for Codegen {
    fn drop(&mut self) {
        unsafe {
            LLVMDisposeBuilder(self.builder);
            LLVMDisposeModule(self.module);
            LLVMContextDispose(self.context);
        }
    }
}

impl Codegen {
    pub unsafe fn try_new() -> Result<Codegen, CodegenError> {
        let context = LLVMContextCreate();
        let module = LLVMModuleCreateWithNameInContext(cstr!("soft"), context);
        let builder = LLVMCreateBuilderInContext(context);
        let types = types::Types::try_new(context)?;

        Ok(Codegen {
            context,
            module,
            builder,
            types,
        })
    }

    pub unsafe fn install_error_handling(self) -> Self {
        // enable diagnostic messages
        let diagnostic_context = LLVMContextGetDiagnosticContext(self.context);
        LLVMContextSetDiagnosticHandler(self.context, Some(handle_diagnostic), diagnostic_context);

        self
    }

    pub unsafe fn verify_module(&self) -> Result<(), String> {
        let mut err = std::mem::zeroed();

        if LLVMVerifyModule(self.module, LLVMReturnStatusAction, &mut err) == 1 {
            let message = CStr::from_ptr(err).to_string_lossy().to_string();
            LLVMDisposeErrorMessage(err);
            return Err(message);
        }

        Ok(())
    }

    pub unsafe fn dump_module(&self) {
        LLVMDumpModule(self.module);
    }

    pub unsafe fn install_execution_targets() {
        LLVMLinkInMCJIT();
        LLVM_InitializeNativeTarget();
        LLVM_InitializeNativeAsmPrinter();
    }
}

pub extern "C" fn handle_diagnostic(info: LLVMDiagnosticInfoRef, _context: *mut c_void) {
    unsafe {
        let kind = match LLVMGetDiagInfoSeverity(info) {
            LLVMDSError => "error",
            LLVMDSWarning => "warning",
            LLVMDSRemark => "remark",
            LLVMDSNote => "note",
        };

        let message = CStr::from_ptr(LLVMGetDiagInfoDescription(info)).to_string_lossy();

        println!("[{kind}] {message}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_codegen() {
        unsafe {
            Codegen::install_execution_targets();

            let codegen = Codegen::try_new().unwrap().install_error_handling();

            let args_t = [codegen.types.u64, codegen.types.u64].as_mut_ptr();
            let sum_t = LLVMFunctionType(codegen.types.u64, args_t, 2, 0);
            let sum = LLVMAddFunction(codegen.module, cstr!("sum"), sum_t);

            let entry = LLVMAppendBasicBlockInContext(codegen.context, sum, cstr!("entry"));
            LLVMPositionBuilderAtEnd(codegen.builder, entry);

            let x = LLVMGetParam(sum, 0);
            let y = LLVMGetParam(sum, 1);

            let value = LLVMBuildAdd(codegen.builder, x, y, cstr!("sum.value"));
            LLVMBuildRet(codegen.builder, value);

            codegen.dump_module();
            codegen.verify_module().unwrap_or_else(|error| {
                for line in error.split("\n") {
                    println!("[error*] {}", line);
                }

                panic!("Module verification failed")
            });

            let engine = execution::ExecutionEngine::try_new(codegen.module).unwrap();

            let f: extern "C" fn(u64, u64) -> u64 =
                std::mem::transmute(engine.get_function_address("sum"));

            println!("sum(1, 2): {} = {}", types::Type(sum_t), f(1, 2));
        }
    }
}
