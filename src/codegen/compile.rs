use im::HashMap;

use crate::specialized::Term;
use llvm_sys::LLVMIntPredicate::LLVMIntEQ;
use std::mem::MaybeUninit;

use super::*;

#[derive(PartialEq, Eq, Clone)]
pub struct SymbolRef {
    pub value_type: LLVMTypeRef,
    pub value: LLVMValueRef,
    pub addr: *mut libc::c_void,
    pub arity: Option<u32>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum CompileError {}

impl Codegen {
    pub unsafe fn compile_main(&mut self, term: Term) -> Result<(), CompileError> {
        let main_t = LLVMFunctionType(self.types.ptr, [].as_mut_ptr(), 0, 0);
        let main = LLVMAddFunction(self.module, cstr!("main"), main_t);

        let entry = LLVMAppendBasicBlockInContext(self.context, main, cstr!("entry"));
        LLVMPositionBuilderAtEnd(self.builder, entry);

        self.symbols.current = main;

        let value = self.compile_term(term)?;
        LLVMBuildRet(self.builder, value);

        Ok(())
    }

    pub unsafe fn compile_term(&self, term: Term) -> Result<LLVMValueRef, CompileError> {
        let current = self.symbols.current;

        match term {
            Term::Lam(_, _, _) => todo!(),
            Term::Let(_, _) => todo!(),
            Term::App(_, _) => todo!(),
            Term::Closure(_, _) => todo!(),
            Term::EnvRef(_) => todo!(),
            Term::Set(_, _, _) => todo!(),
            Term::Call(_, _) => todo!(),
            Term::LocalRef(_) => todo!(),
            Term::GlobalRef(_) => todo!(),
            Term::Num(n) => {
                let x = LLVMConstInt(self.types.u64, n as u64, 0);
                Ok(self.call_prim("prim__Value_new_num", &mut [x]))
            }
            Term::Quote(_) => todo!(),
            Term::Nil => Ok(self.call_prim("prim__Value_nil", &mut [])),
            Term::If(box cond_term, box then_term, box else_term) => {
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
            Term::Cons(_, _) => todo!(),
        }
    }

    unsafe fn build_if_true(&self, cond: LLVMValueRef) -> LLVMValueRef {
        let is_true = self.call_prim("prim__Value_is_true", &mut [cond]);
        let true_v = LLVMConstInt(self.types.i1, 1, 0);

        LLVMBuildICmp(self.builder, LLVMIntEQ, is_true, true_v, cstr!())
    }

    unsafe fn call_prim(&self, name: &str, args: &mut [LLVMValueRef]) -> LLVMValueRef {
        let symbol_ref = self
            .symbols
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

pub struct Context {
    pub module: LLVMModuleRef,
    pub symbols: HashMap<String, SymbolRef>,
    pub current: LLVMValueRef,
}

impl From<LLVMModuleRef> for Context {
    fn from(module: LLVMModuleRef) -> Self {
        Self {
            module,
            symbols: HashMap::new(),
            current: unsafe { MaybeUninit::zeroed().assume_init() },
        }
    }
}

type FunctionRef<'a> = (&'a str, *mut libc::c_void);

impl Context {
    pub fn with_sym(&mut self, f: FunctionRef, return_t: LLVMTypeRef, args: &mut [LLVMTypeRef]) {
        let (name, addr) = f;

        let func_t = unsafe { LLVMFunctionType(return_t, args.as_mut_ptr(), args.len() as _, 0) };
        let func = unsafe { LLVMAddFunction(self.module, cstr!(name), func_t) };
        let symbol_ref = SymbolRef {
            value_type: func_t,
            value: func,
            addr,
            arity: Some(args.len() as _),
        };

        self.symbols.insert(name.to_string(), symbol_ref);
    }
}
