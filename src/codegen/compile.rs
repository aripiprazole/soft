use im::HashMap;

use crate::specialized::Term;

use super::*;

#[derive(PartialEq, Eq, Clone)]
pub struct SymbolRef(pub LLVMTypeRef, pub LLVMValueRef, pub *mut libc::c_void);

#[derive(Default)]
pub struct Context {
    pub symbols: HashMap<String, SymbolRef>,
}

impl Codegen {
    pub unsafe fn compile_main(&self, ctx: &mut Context, term: Term) {
        use crate::runtime::primitives::value::*;

        let main_t = LLVMFunctionType(self.types.void_ptr, [].as_mut_ptr(), 0, 0);
        let main = LLVMAddFunction(self.module, cstr!("main"), main_t);

        let new_num_t = LLVMFunctionType(self.types.void_ptr, [self.types.u64].as_mut_ptr(), 1, 0);
        let new_num = LLVMAddFunction(self.module, cstr!("prim__Value_new_num"), new_num_t);

        ctx.symbols.insert(
            "prim__Value_new_num".to_string(),
            SymbolRef(new_num_t, new_num, prim__Value_new_num as *mut _),
        );

        let entry = LLVMAppendBasicBlockInContext(self.context, main, cstr!("entry"));
        LLVMPositionBuilderAtEnd(self.builder, entry);

        let value = self.compile_term(ctx, term);
        LLVMBuildRet(self.builder, value);
    }

    pub unsafe fn compile_term(&self, ctx: &mut Context, term: Term) -> LLVMValueRef {
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
                self.call_prim(ctx, "prim__Value_new_num", &mut [x])
            }
            Term::Quote(_) => todo!(),
            Term::Nil => todo!(),
        }
    }

    unsafe fn call_prim(
        &self,
        ctx: &mut Context,
        name: &str,
        args: &mut [LLVMValueRef],
    ) -> LLVMValueRef {
        let SymbolRef(func_t, func, _) = *ctx
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
