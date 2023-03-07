use im::HashMap;

use crate::specialized::Term;
use llvm_sys::LLVMIntPredicate::LLVMIntEQ;

use super::*;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum CompileError {
    UndefinedLocalRef(String),
}

impl Codegen {
    pub unsafe fn compile_main(&mut self, term: Term) -> Result<(), CompileError> {
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

    pub unsafe fn compile_term(&mut self, term: Term) -> Result<LLVMValueRef, CompileError> {
        use Term::*;

        let current = self.current_fn;

        match term {
            Lam(_, _, _) => todo!(),
            Let(entries, box body) => {
                self.enter_scope();

                for (name, value) in entries {
                    let value = self.compile_term(value)?;
                    let alloca = LLVMBuildAlloca(self.builder, self.types.ptr, cstr!(name));
                    LLVMBuildStore(self.builder, value, alloca);

                    let symbol_ref = SymbolRef::new(self.types.ptr, alloca);
                    self.environment.symbols.insert(name, symbol_ref);
                }

                let body = self.compile_term(body)?;

                self.pop_scope();

                Ok(body)
            }
            App(_, _) => todo!(),
            Closure(_, _) => todo!(),
            EnvRef(_) => todo!(),
            Set(_, _, _) => todo!(),
            Call(_, _) => todo!(),
            LocalRef(sym) => {
                let symbol = self
                    .environment
                    .symbols
                    .get(&sym)
                    .ok_or_else(|| CompileError::UndefinedLocalRef(sym))?;

                let value = LLVMBuildLoad2(self.builder, symbol.value_type, symbol.value, cstr!());
                Ok(value)
            }
            GlobalRef(_) => todo!(),
            Num(n) => {
                let x = LLVMConstInt(self.types.u64, n as u64, 0);
                Ok(self.call_prim("prim__Value_new_num", &mut [x]))
            }
            Quote(_) => todo!(),
            Cons(box head, box tail) => {
                let head = self.compile_term(head)?;
                let tail = self.compile_term(tail)?;

                Ok(self.call_prim("prim__Value_cons", &mut [head, tail]))
            }
            Nil => Ok(self.call_prim("prim__Value_nil", &mut [])),
            If(box cond_term, box then_term, box else_term) => {
                let next_br = LLVMAppendBasicBlockInContext(self.context, current, cstr!());
                let then_br = LLVMAppendBasicBlockInContext(self.context, current, cstr!());
                let else_br = LLVMAppendBasicBlockInContext(self.context, current, cstr!());

                let cond_value = self.compile_term(cond_term)?;
                let cond = self.build_if_true(cond_value);

                LLVMBuildCondBr(self.builder, cond, then_br, else_br);

                LLVMPositionBuilderAtEnd(self.builder, then_br);

                let then_value = self.compile_term(then_term)?;
                LLVMBuildBr(self.builder, next_br);

                LLVMPositionBuilderAtEnd(self.builder, else_br);

                let else_value = self.compile_term(else_term)?;
                LLVMBuildBr(self.builder, next_br);

                LLVMPositionBuilderAtEnd(self.builder, next_br);

                let phi = LLVMBuildPhi(self.builder, self.types.ptr, cstr!());

                LLVMAddIncoming(
                    phi,
                    [then_value, else_value].as_mut_ptr(),
                    [then_br, else_br].as_mut_ptr(),
                    2,
                );

                Ok(phi)
            }
        }
    }

    unsafe fn delete_main_if_exists(&self) {
        let main = LLVMGetNamedFunction(self.module, cstr!("main"));

        if !main.is_null() {
            LLVMDeleteFunction(main);
        }
    }

    unsafe fn build_if_true(&self, cond: LLVMValueRef) -> LLVMValueRef {
        let is_true = self.call_prim("prim__Value_is_true", &mut [cond]);
        let true_v = LLVMConstInt(self.types.i1, 1, 0);

        LLVMBuildICmp(self.builder, LLVMIntEQ, is_true, true_v, cstr!())
    }

    unsafe fn call_prim(&self, name: &str, args: &mut [LLVMValueRef]) -> LLVMValueRef {
        let symbol_ref = self
            .environment
            .symbols
            .get(name)
            .expect(&format!("No such primitive: {name}"));

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

#[derive(PartialEq, Eq, Clone)]
pub struct SymbolRef {
    pub value_type: LLVMTypeRef,
    pub value: LLVMValueRef,
    pub addr: *mut libc::c_void,
    pub arity: Option<u16>,
}

impl SymbolRef {
    pub unsafe fn new(value_type: LLVMTypeRef, value: LLVMValueRef) -> Self {
        Self {
            value_type,
            value,
            addr: std::mem::zeroed(),
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
    pub super_environment: Box<Option<Environment>>,
}

impl From<LLVMModuleRef> for Environment {
    fn from(module: LLVMModuleRef) -> Self {
        Self {
            module,
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
            symbols: self.environment.symbols.clone(),
            super_environment: box Some(self.environment.clone()),
        };
    }

    pub fn pop_scope(&mut self) {
        self.environment = self.environment.super_environment.clone().unwrap();
    }
}

impl Environment {
    pub fn with_sym(&mut self, f: FunctionRef, return_t: LLVMTypeRef, args: &mut [LLVMTypeRef]) {
        let (name, addr) = f;

        let func_t = unsafe { LLVMFunctionType(return_t, args.as_mut_ptr(), args.len() as _, 0) };
        let func_v = unsafe { LLVMAddFunction(self.module, cstr!(name), func_t) };
        let symbol_ref = SymbolRef {
            value_type: func_t,
            value: func_v,
            addr,
            arity: Some(args.len() as _),
        };

        self.symbols.insert(name.to_string(), symbol_ref);
    }
}
