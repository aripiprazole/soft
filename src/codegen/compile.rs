use im::HashMap;

use crate::specialized::Term;
use llvm_sys::LLVMIntPredicate::LLVMIntEQ;
use std::mem::MaybeUninit;

use super::*;

#[derive(PartialEq, Eq, Clone)]
pub struct SymbolRef(pub LLVMTypeRef, pub LLVMValueRef, pub *mut libc::c_void);

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
        let symbol_ref = SymbolRef(func_t, func, addr);

        self.symbols.insert(name.to_string(), symbol_ref);
    }
}

impl Codegen {
    pub unsafe fn compile_main(&mut self, term: Term) {
        let main_t = LLVMFunctionType(self.types.ptr, [].as_mut_ptr(), 0, 0);
        let main = LLVMAddFunction(self.module, cstr!("main"), main_t);

        let entry = LLVMAppendBasicBlockInContext(self.context, main, cstr!("entry"));
        LLVMPositionBuilderAtEnd(self.builder, entry);

        self.symbols.current = main;

        let value = self.compile_term(term);
        LLVMBuildRet(self.builder, value);
    }

    pub unsafe fn compile_term(&self, term: Term) -> LLVMValueRef {
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
                self.call_prim("prim__Value_new_num", &mut [x])
            }
            Term::Quote(_) => todo!(),
            Term::Nil => self.call_prim("prim__Value_nil", &mut []),
            Term::If(cond_term, then_term, else_term) => {
                let next_br = LLVMAppendBasicBlockInContext(self.context, current, cstr!());
                let then_br = LLVMAppendBasicBlockInContext(self.context, current, cstr!());
                let else_br = LLVMAppendBasicBlockInContext(self.context, current, cstr!());

                let cond_value = self.compile_term(*cond_term);

                let is_true = self.call_prim("prim__Value_is_true", &mut [cond_value]);
                let true_v = LLVMConstInt(self.types.i1, 1, 0);

                let result = LLVMBuildICmp(self.builder, LLVMIntEQ, is_true, true_v, cstr!());

                LLVMBuildCondBr(self.builder, result, then_br, else_br);

                LLVMPositionBuilderAtEnd(self.builder, then_br);

                let then_value = self.compile_term(*then_term);
                LLVMBuildBr(self.builder, next_br);

                LLVMPositionBuilderAtEnd(self.builder, else_br);

                let else_value = self.compile_term(*else_term);
                LLVMBuildBr(self.builder, next_br);

                LLVMPositionBuilderAtEnd(self.builder, next_br);

                let phi = LLVMBuildPhi(self.builder, self.types.ptr, cstr!());

                LLVMAddIncoming(
                    phi,
                    [then_value, else_value].as_mut_ptr(),
                    [then_br, else_br].as_mut_ptr(),
                    2,
                );

                phi
            }
            Term::Cons(_, _) => todo!(),
        }
    }

    unsafe fn call_prim(&self, name: &str, args: &mut [LLVMValueRef]) -> LLVMValueRef {
        let SymbolRef(func_t, func, _) = *self
            .symbols
            .symbols
            .get(name)
            .expect(&format!("No such primitive: {name}"));

        LLVMBuildCall2(
            self.builder,
            func_t,
            func,
            args.as_mut_ptr(),
            args.len() as u32,
            cstr!(),
        )
    }
}
