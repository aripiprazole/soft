use super::*;

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
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    pub fn try_new(module: LLVMModuleRef) -> Result<Self, CodegenError> {
        unsafe {
            let mut ptr = std::mem::MaybeUninit::uninit();
            let mut err = std::mem::zeroed();

            if LLVMCreateExecutionEngineForModule(ptr.as_mut_ptr(), module, &mut err) != 0 {
                // In case of error, we must avoid using the uninitialized ExecutionEngineRef.
                assert!(!err.is_null());
                let err = CStr::from_ptr(err);
                return Err(format!("Failed to create execution engine: {:?}", err));
            }

            Ok(ExecutionEngine(ptr.assume_init()))
        }
    }

    pub fn install_primitive_symbols(self, context: &compile::Environment) -> Self {
        unsafe {
            for symbol_ref in context.symbols.values() {
                LLVMAddGlobalMapping(self.0, symbol_ref.value, symbol_ref.addr);
            }

            self
        }
    }

    pub fn install_global_environment(self, codegen: &Codegen) -> Self {
        unsafe {
            let global_environment = codegen.global_environment as *mut libc::c_void;
            let global_sym = LLVMGetNamedGlobal(codegen.module, cstr!("global"));

            LLVMAddGlobalMapping(self.0, global_sym, global_environment);

            self
        }
    }

    pub fn get_function_address(&self, name: &str) -> u64 {
        unsafe { LLVMGetFunctionAddress(self.0, cstr!(name)) }
    }
}
