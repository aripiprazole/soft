use im::HashMap;

use crate::specialized::Term;
use llvm_sys::LLVMIntPredicate::LLVMIntEQ;

use super::*;

pub type Result<T = LLVMValueRef> = std::result::Result<T, CompileError>;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum CompileError {
    UndefinedEnvRef(String),
    UndefinedLocalRef(String),
    UndefinedGlobalRef(String),
}

impl Codegen {
    pub fn compile_main(&mut self, term: Term) -> Result<()> {
        unsafe {
            self.delete_main_if_exists();

            let main_t = LLVMFunctionType(self.types.ptr, [].as_mut_ptr(), 0, 0);
            let main = LLVMAddFunction(self.module, cstr!("main"), main_t);

            let entry = LLVMAppendBasicBlockInContext(self.context, main, cstr!("entry"));
            LLVMPositionBuilderAtEnd(self.builder, entry);

            self.current_fn = main;

            let value = self.compile_term(term)?;
            LLVMBuildRet(self.builder, value);

            Ok(())
        }
    }

    pub fn make_if(&self, cond: LLVMValueRef) -> LLVMValueRef {
        unsafe {
            let is_true = self.make_call("prim__Value_is_true", &mut [cond]);
            let true_v = LLVMConstInt(self.types.i1, 1, 0);

            LLVMBuildICmp(self.builder, LLVMIntEQ, is_true, true_v, cstr!())
        }
    }

    pub fn make_call(&self, name: &str, args: &mut [LLVMValueRef]) -> LLVMValueRef {
        unsafe {
            let symbol_ref = self
                .environment
                .symbols
                .get(name)
                .unwrap_or_else(|| panic!("No such primitive: {name}"));

            LLVMBuildCall2(
                self.builder,
                symbol_ref.value_type,
                symbol_ref.value,
                args.as_mut_ptr(),
                args.len() as u32,
                cstr!(),
            )
        }
    }

    unsafe fn delete_main_if_exists(&self) {
        unsafe {
            let main = LLVMGetNamedFunction(self.module, cstr!("main"));

            if !main.is_null() {
                LLVMDeleteFunction(main);
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct SymbolRef {
    pub value_type: LLVMTypeRef,
    pub value: LLVMValueRef,
    pub addr: *mut libc::c_void,
    pub arity: Option<u16>,
}

impl SymbolRef {
    pub fn new(value_type: LLVMTypeRef, value: LLVMValueRef) -> Self {
        Self {
            value_type,
            value,
            addr: std::ptr::null_mut(),
            arity: None,
        }
    }

    pub fn with_arity(mut self, arity: u16) -> Self {
        self.arity = Some(arity);
        self
    }
}

#[derive(Clone)]
pub struct Environment {
    pub module: LLVMModuleRef,
    pub symbols: HashMap<String, SymbolRef>,
    pub closure: HashMap<String, usize>,
    pub super_environment: Box<Option<Environment>>,
}

impl From<LLVMModuleRef> for Environment {
    fn from(module: LLVMModuleRef) -> Self {
        Self {
            module,
            closure: HashMap::new(),
            symbols: HashMap::new(),
            super_environment: box None,
        }
    }
}

type FunctionRef<'a> = (&'a str, *mut libc::c_void);

impl Codegen {
    pub fn enter_scope(&mut self) {
        self.environment = Environment {
            module: self.module,
            closure: self.environment.closure.clone(),
            symbols: self.environment.symbols.clone(),
            super_environment: box Some(self.environment.clone()),
        };
    }

    pub fn pop_scope(&mut self) {
        self.environment = self.environment.super_environment.clone().unwrap();
    }
}

impl Environment {
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    pub fn with<const N: usize>(
        &mut self,
        function_ref: FunctionRef,
        return_type: LLVMTypeRef,
        mut args: [LLVMTypeRef; N],
    ) {
        let (name, addr) = function_ref;

        unsafe {
            let value_type = LLVMFunctionType(return_type, args.as_mut_ptr(), args.len() as _, 0);
            let value = LLVMAddFunction(self.module, cstr!(name), value_type);
            let symbol_ref = SymbolRef {
                value_type,
                value,
                addr,
                arity: Some(args.len() as _),
            };

            self.symbols.insert(name.to_string(), symbol_ref);
        }
    }
}
