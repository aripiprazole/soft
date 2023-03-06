use std::{
    ffi::{c_void, CStr},
    mem,
};

use llvm_sys::{
    analysis::{LLVMVerifierFailureAction::LLVMReturnStatusAction, LLVMVerifyModule},
    error::LLVMDisposeErrorMessage,
    error_handling::{
        LLVMEnablePrettyStackTrace, LLVMInstallFatalErrorHandler, LLVMResetFatalErrorHandler,
    },
    target::{LLVM_InitializeNativeAsmPrinter, LLVM_InitializeNativeTarget},
    LLVMDiagnosticSeverity::{LLVMDSError, LLVMDSNote, LLVMDSRemark, LLVMDSWarning},
};
pub use llvm_sys::{core::*, execution_engine::*, prelude::*};

use crate::util::cstr;

pub type CodegenError = String;

pub struct Codegen {
    pub context: LLVMContextRef,
    pub module: LLVMModuleRef,
    pub builder: LLVMBuilderRef,
    pub types: Types,
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
        let types = Types::try_new(context)?;

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
        let mut err = mem::zeroed();

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

pub struct ExecutionEngine(pub *mut LLVMOpaqueExecutionEngine);

impl Drop for ExecutionEngine {
    fn drop(&mut self) {
        // TODO
        // unsafe {
        //     LLVMDisposeExecutionEngine(self.0);
        // }
    }
}

impl ExecutionEngine {
    pub unsafe fn try_new(module: LLVMModuleRef) -> Result<Self, CodegenError> {
        let mut ptr = mem::MaybeUninit::uninit();
        let mut err = mem::zeroed();

        if LLVMCreateExecutionEngineForModule(ptr.as_mut_ptr(), module, &mut err) != 0 {
            // In case of error, we must avoid using the uninitialized ExecutionEngineRef.
            assert!(!err.is_null());
            let err = CStr::from_ptr(err);
            return Err(format!("Failed to create execution engine: {:?}", err));
        }

        Ok(ExecutionEngine(ptr.assume_init()))
    }

    pub unsafe fn get_function_address(&self, name: &str) -> u64 {
        LLVMGetFunctionAddress(self.0, format!("{name}\0").as_ptr() as *const i8)
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

            let engine = ExecutionEngine::try_new(codegen.module).unwrap();

            let f: extern "C" fn(u64, u64) -> u64 =
                std::mem::transmute(engine.get_function_address("sum"));

            println!("sum(1, 2) = {}", f(1, 2));
        }
    }
}
