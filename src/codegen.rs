use std::ffi::{c_void, CStr};

use llvm_sys::{
    analysis::{LLVMVerifierFailureAction::LLVMReturnStatusAction, LLVMVerifyModule},
    error::LLVMDisposeErrorMessage,
    target::{LLVM_InitializeNativeAsmPrinter, LLVM_InitializeNativeTarget},
    LLVMDiagnosticSeverity::{LLVMDSError, LLVMDSNote, LLVMDSRemark, LLVMDSWarning},
};

use crate::{
    cli::Options,
    macros::{cstr, llvm_wrapper},
};

pub use llvm_sys::{core::*, execution_engine::*, prelude::*};

use self::jit::GlobalEnvironment;

pub mod compile;
pub mod execution;
pub mod helpers;
pub mod jit;
pub mod term;
pub mod types;

pub type CodegenError = String;

pub struct Codegen {
    pub context: LLVMContextRef,
    pub module: LLVMModuleRef,
    pub builder: LLVMBuilderRef,
    pub types: types::Types,
    pub current_fn: LLVMValueRef,
    pub environment: compile::Environment,
    pub global_environment: *mut GlobalEnvironment,
    pub global_sym: LLVMValueRef,
    pub options: Box<crate::cli::Options>,
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
    pub fn new(global_environment: *mut GlobalEnvironment) -> Codegen {
        unsafe {
            let context = LLVMContextCreate();
            let module = LLVMModuleCreateWithNameInContext(cstr!("soft"), context);
            let builder = LLVMCreateBuilderInContext(context);
            let types = types::Types::from(context);
            let environment = compile::Environment::from(module);

            Self {
                context,
                module,
                builder,
                types,
                environment,
                global_environment,
                current_fn: std::ptr::null_mut(),
                global_sym: std::ptr::null_mut(),
                options: Box::default(),
            }
        }
    }

    pub fn install_error_handling(mut self) -> Self {
        unsafe {
            // enable diagnostic messages
            let diagnostic_context = self.options.as_mut() as *mut _ as *mut c_void;
            let handle_diagnostic = handle_diagnostic as extern "C" fn(_, *mut c_void);
            let handle_fn: Option<extern "C" fn(_, _)> = Some(handle_diagnostic);
            LLVMContextSetDiagnosticHandler(self.context, handle_fn, diagnostic_context);

            self
        }
    }

    pub fn install_global_environment(mut self) -> Self {
        unsafe {
            self.global_sym = LLVMAddGlobal(self.module, self.types.ptr, cstr!("global"));
            self
        }
    }

    pub fn verify_module(&self) -> Result<(), String> {
        unsafe {
            let mut err = std::mem::zeroed();

            if LLVMVerifyModule(self.module, LLVMReturnStatusAction, &mut err) == 1 {
                let message = CStr::from_ptr(err).to_string_lossy().to_string();
                LLVMDisposeErrorMessage(err);
                return Err(message);
            }

            Ok(())
        }
    }

    pub fn dump_module(&self) {
        unsafe {
            LLVMDumpModule(self.module);
        }
    }

    pub fn install_execution_targets() {
        unsafe {
            LLVMLinkInMCJIT();
            LLVM_InitializeNativeTarget();
            LLVM_InitializeNativeAsmPrinter();
        }
    }
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn handle_diagnostic(info: LLVMDiagnosticInfoRef, context: *mut libc::c_void) {
    let options = context as *mut Options;

    unsafe {
        if !(*options).debug {
            return;
        }

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
    use crate::{runtime::ValueRef, specialized::Term};

    use super::*;

    #[test]
    fn test_codegen() {
        Codegen::install_execution_targets();

        let global_environment = Box::leak(Box::default());

        let mut codegen = Codegen::new(global_environment)
            .install_error_handling()
            .install_primitives()
            .install_global_environment();

        codegen.compile_main(Term::Num(42)).unwrap();
        codegen.dump_module();
        codegen.verify_module().unwrap_or_else(|error| {
            for line in error.split('\n') {
                println!("[error*] {}", line);
            }

            panic!("Module verification failed")
        });

        let engine = execution::ExecutionEngine::try_new(codegen.module)
            .unwrap()
            .install_primitive_symbols(&codegen.environment)
            .install_global_environment(&codegen);

        let f: extern "C" fn() -> ValueRef =
            unsafe { std::mem::transmute(engine.get_function_address("main")) };

        println!("main() = {}", f());
    }
}
