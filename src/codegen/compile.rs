use im::HashMap;

use crate::{runtime::primitives::AnyPtr, specialized::Term};
use llvm_sys::LLVMIntPredicate::LLVMIntEQ;

use super::{
    builder::{IRBuilder, IRContext, IRModule},
    *,
};

pub type Result<T = LLVMValueRef> = std::result::Result<T, CompileError>;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum CompileError {
    UndefinedEnvRef(String),
    UndefinedLocalRef(String),
    UndefinedGlobalRef(String),
}

impl Codegen {
    pub fn compile_main(&mut self, term: Term) -> Result<()> {
        self.delete_main_if_exists();

        let main = self.module.add_function("main", &mut [], self.types.ptr);
        let entry = self.context.append_basic_block(main, "entry");

        self.builder.position_at_end(entry);
        self.current_fn = main;

        let value = self.compile_term(term)?;
        self.builder.build_ret(value);

        Ok(())
    }

    pub fn make_if(&self, cond: LLVMValueRef) -> LLVMValueRef {
        let is_true = self.make_call("prim__Value_is_true", &mut [cond]);
        let true_value = self.true_value();

        self.builder.build_icmp(LLVMIntEQ, is_true, true_value, "")
    }

    pub fn make_call(&self, name: &str, args: &mut [LLVMValueRef]) -> LLVMValueRef {
        let symbol_ref = self
            .environment
            .symbols
            .get(name)
            .unwrap_or_else(|| panic!("No such primitive: {name}"));

        self.builder
            .build_call(symbol_ref.kind, symbol_ref.value, args, "")
    }

    pub fn enter_scope(&mut self) {
        self.environment = Environment {
            module: self.module,
            closure: self.environment.closure.clone(),
            symbols: self.environment.symbols.clone(),
            super_environment: Box::new(Some(self.environment.clone())),
        };
    }

    pub fn pop_scope(&mut self) {
        self.environment = self.environment.super_environment.clone().unwrap();
    }

    fn delete_main_if_exists(&self) {
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
    pub kind: LLVMTypeRef,
    pub value: LLVMValueRef,
    pub addr: AnyPtr,
    pub arity: Option<u16>,
}

impl SymbolRef {
    pub fn new(value_type: LLVMTypeRef, value: LLVMValueRef) -> Self {
        Self {
            kind: value_type,
            value,
            addr: std::ptr::null_mut(),
            arity: None,
        }
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
            super_environment: Box::new(None),
        }
    }
}

type FunctionRef<'a> = (&'a str, AnyPtr);

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
                kind: value_type,
                value,
                addr,
                arity: Some(args.len() as _),
            };

            self.symbols.insert(name.to_string(), symbol_ref);
        }
    }
}
