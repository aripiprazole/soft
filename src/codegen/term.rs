use super::{compile::Result, *};
use crate::{
    codegen::compile::{CompileError, SymbolRef},
    specialized::Term,
};

impl Codegen {
    pub fn compile_term(&mut self, term: Term) -> Result {
        use Term::*;

        let current_fn = self.current_fn;

        unsafe {
            match term {
                Quote(_) => todo!(),
                Nil => Ok(self.make_call("prim__Value_nil", &mut [])),
                Num(n) => {
                    let x = LLVMConstInt(self.types.u64, n, 0);
                    Ok(self.make_call("prim__Value_new_num", &mut [x]))
                }
                Cons(box head, box tail) => {
                    let head = self.compile_term(head)?;
                    let tail = self.compile_term(tail)?;

                    Ok(self.make_call("prim__Value_cons", &mut [head, tail]))
                }
                Set(name, _, box value_term) => {
                    let sym = LLVMBuildGlobalStringPtr(self.builder, cstr!(name), cstr!());
                    let value = self.compile_term(value_term)?;

                    self.make_call("prim__global_set", &mut [self.global_sym, sym, value]);

                    self.compile_term(Term::Nil)
                }
                GlobalRef(sym) => {
                    let sym_lit = LLVMBuildGlobalStringPtr(self.builder, cstr!(sym), cstr!());

                    let value = self.make_call("prim__global_get", &mut [self.global_sym, sym_lit]);

                    Ok(value)
                }
                LocalRef(sym) => {
                    let symbol = self
                        .environment
                        .symbols
                        .get(&sym)
                        .ok_or(CompileError::UndefinedLocalRef(sym))?;

                    let value =
                        LLVMBuildLoad2(self.builder, symbol.value_type, symbol.value, cstr!());
                    Ok(value)
                }
                EnvRef(sym) => {
                    let index = self
                        .environment
                        .closure
                        .get(&sym)
                        .ok_or_else(|| CompileError::UndefinedEnvRef(sym.clone()))?;

                    let env_param = LLVMGetLastParam(current_fn);

                    let index_value = LLVMConstInt(self.types.u64, *index as _, 0);
                    let value = self.make_call("prim__Value_gep", &mut [env_param, index_value]);

                    Ok(value)
                }
                Closure(env, box term) => self.compile_closure(env, term),
                Let(entries, box term) => self.compile_let(entries, term),
                If(box cond, box then, box otherwise) => self.compile_if(cond, then, otherwise),
                Lam(_, args, box term) => self.compile_lam(args, term),
                App(box callee, args) => self.compile_app(callee, args),
            }
        }
    }

    fn compile_closure(&mut self, env: Vec<(String, Term)>, term: Term) -> Result {
        self.enter_scope();

        let closure_environment = env
            .iter()
            .enumerate()
            .map(|(index, (name, _))| (name.clone(), index))
            .collect();

        self.environment.closure = closure_environment;

        let closure = self.make_closure(env, term)?;

        self.pop_scope();

        Ok(closure)
    }

    fn compile_let(&mut self, entries: Vec<(String, Term)>, term: Term) -> Result {
        unsafe {
            self.enter_scope();

            for (name, value) in entries {
                let value = self.compile_term(value)?;
                let alloca = LLVMBuildAlloca(self.builder, self.types.ptr, cstr!(name));
                LLVMBuildStore(self.builder, value, alloca);

                let symbol_ref = SymbolRef::new(self.types.ptr, alloca);
                self.environment.symbols.insert(name, symbol_ref);
            }

            let value = self.compile_term(term)?;

            self.pop_scope();

            Ok(value)
        }
    }

    fn compile_if(&mut self, cond: Term, then: Term, otherwise: Term) -> Result {
        unsafe {
            let current_fn = self.current_fn;

            let next_br = LLVMAppendBasicBlockInContext(self.context, current_fn, cstr!());
            let then_br = LLVMAppendBasicBlockInContext(self.context, current_fn, cstr!());
            let else_br = LLVMAppendBasicBlockInContext(self.context, current_fn, cstr!());

            let cond_value = self.compile_term(cond)?;
            let cond_value = self.make_if(cond_value);

            LLVMBuildCondBr(self.builder, cond_value, then_br, else_br);

            LLVMPositionBuilderAtEnd(self.builder, then_br);

            let then_value = self.compile_term(then)?;
            LLVMBuildBr(self.builder, next_br);

            LLVMPositionBuilderAtEnd(self.builder, else_br);

            let else_value = self.compile_term(otherwise)?;
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

    fn compile_lam(&mut self, args: Vec<String>, term: Term) -> Result {
        unsafe {
            let old_block = LLVMGetInsertBlock(self.builder);
            let current_fn = self.current_fn;

            self.enter_scope();

            let mut arg_types = vec![self.types.ptr; args.len()];
            let function_type =
                LLVMFunctionType(self.types.ptr, arg_types.as_mut_ptr(), args.len() as _, 0);

            let new_fn = LLVMAddFunction(self.module, cstr!(), function_type);
            self.current_fn = new_fn;

            let entry = LLVMAppendBasicBlock(new_fn, cstr!("entry"));
            LLVMPositionBuilderAtEnd(self.builder, entry);

            for (index, name) in args.iter().enumerate() {
                let alloca = LLVMBuildAlloca(self.builder, self.types.ptr, cstr!());
                let arg_value = LLVMGetParam(new_fn, index as _);
                LLVMBuildStore(self.builder, arg_value, alloca);

                let symbol_ref = SymbolRef::new(self.types.ptr, alloca);
                self.environment.symbols.insert(name.clone(), symbol_ref);
            }

            let body = self.compile_term(term)?;
            LLVMBuildRet(self.builder, body);

            self.pop_scope();
            self.current_fn = current_fn;
            LLVMPositionBuilderAtEnd(self.builder, old_block);

            let arity = LLVMConstInt(self.types.u64, args.len() as _, 0);
            let new_fn = LLVMBuildPointerCast(
                self.builder,
                new_fn,
                LLVMPointerType(self.types.ptr, 0),
                cstr!(),
            );

            Ok(self.make_call("prim__Value_function", &mut [arity, new_fn]))
        }
    }

    // IMPERATIVE LOGIC:
    // given fn;
    // let address = fn-address(fn);
    // if address != null {
    //     if !fn-arity(fn) == args.len() {
    //         panic!(bla)
    //     } else {
    //         let fn-ptr = cast (ptr (...args)) address;
    //         return fn-ptr(...args)
    //     }
    // } else {
    //     let env = closure-get-env(fn);
    //     if env != null {
    //         address = closure-get-fn(fn);
    //         if address != null {
    //             if !fn-arity(fn) == args.len() {
    //                 panic!(bla)
    //             } else {
    //                 let fn-ptr = cast (ptr (...args, #env)) address;
    //                 return fn-ptr(...args, #env)
    //             }
    //         }
    //     }
    //     panic!("not a function")
    // }
    //
    // LLVM LOGIC:
    //     fn = compile-term(callee);
    //     address = fn-address(fn);
    //     res = cmp equal address 0
    //     branch res %else %is_fun
    // %is_fun:
    //     call check_arity(fn, !args.len())
    //     cast
    //     call
    // %else:
    //     env = closure-get-env(fn)
    //     res = cmp equal env 0
    //     branch res %else %closure
    // %closure:
    //      fn' = closure-get-fn(fn);
    //      call check_arity(fn', !args.len() + 1)
    //      cast + #env
    //      call + #env
    // %else:
    //     panic!
    // %next:
    //     %0 = phi [closure, is_fun]
    fn compile_app(&mut self, callee: Term, args: Vec<Term>) -> Result {
        unsafe {
            let current_fn = self.current_fn;

            let is_fun = LLVMAppendBasicBlockInContext(self.context, current_fn, cstr!());
            let else_br = LLVMAppendBasicBlockInContext(self.context, current_fn, cstr!());
            let is_closure = LLVMAppendBasicBlockInContext(self.context, current_fn, cstr!());
            let else_panic = LLVMAppendBasicBlockInContext(self.context, current_fn, cstr!());

            let next = LLVMAppendBasicBlockInContext(self.context, current_fn, cstr!());

            let args_len = LLVMConstInt(self.types.u64, args.len() as _, 0);

            let callee_value = self.compile_term(callee)?;

            let address = self.make_call("prim__check_arity", &mut [callee_value, args_len]);
            let is_true = self.make_call("prim__is_null", &mut [address]);

            LLVMBuildCondBr(self.builder, is_true, else_br, is_fun);

            LLVMPositionBuilderAtEnd(self.builder, is_fun);

            let mut args_t = vec![self.types.ptr; args.len()];
            let call_t = LLVMFunctionType(self.types.ptr, args_t.as_mut_ptr(), args.len() as _, 0);

            let fn_ptr =
                LLVMBuildPointerCast(self.builder, address, LLVMPointerType(call_t, 0), cstr!());

            let mut args_value = vec![];

            for n in args.clone() {
                args_value.push(self.compile_term(n)?);
            }

            let res2 = LLVMBuildCall2(
                self.builder,
                call_t,
                fn_ptr,
                args_value.as_mut_ptr(),
                args_value.len() as u32,
                cstr!(),
            );

            LLVMBuildBr(self.builder, next);

            LLVMPositionBuilderAtEnd(self.builder, else_br);

            let fun = self.make_call("prim__closure_get_fn", &mut [callee_value]);
            let is_true = self.make_call("prim__is_null", &mut [fun]);
            LLVMBuildCondBr(self.builder, is_true, else_panic, is_closure);

            LLVMPositionBuilderAtEnd(self.builder, is_closure);

            let args_len = LLVMConstInt(self.types.u64, (args.len() + 1) as _, 0);
            let address = self.make_call("prim__check_arity", &mut [fun, args_len]);

            let env = self.make_call("prim__closure_get_env", &mut [callee_value]);

            let mut args_value = vec![];

            for n in args {
                args_value.push(self.compile_term(n)?);
            }

            args_value.push(env);

            let mut args_t = vec![self.types.ptr; args_value.len()];
            let call_t = LLVMFunctionType(
                self.types.ptr,
                args_t.as_mut_ptr(),
                args_value.len() as _,
                0,
            );

            let fn_ptr =
                LLVMBuildPointerCast(self.builder, address, LLVMPointerType(call_t, 0), cstr!());

            let res1 = LLVMBuildCall2(
                self.builder,
                call_t,
                fn_ptr,
                args_value.as_mut_ptr(),
                args_value.len() as u32,
                cstr!(),
            );

            LLVMBuildBr(self.builder, next);
            LLVMPositionBuilderAtEnd(self.builder, else_panic);
            LLVMBuildUnreachable(self.builder);
            LLVMPositionBuilderAtEnd(self.builder, next);

            let phi = LLVMBuildPhi(self.builder, self.types.ptr, cstr!());

            LLVMAddIncoming(
                phi,
                [res2, res1].as_mut_ptr(),
                [is_fun, is_closure].as_mut_ptr(),
                2,
            );

            Ok(phi)
        }
    }

    fn make_closure_environment(&mut self, env: Vec<(String, Term)>) -> Result {
        unsafe {
            let env_len = env.len() as u64;
            let env_len = LLVMConstInt(self.types.u64, env_len, 0);

            let env_value = LLVMBuildArrayAlloca(self.builder, self.types.ptr, env_len, cstr!());

            for (index, (_, term)) in env.into_iter().enumerate() {
                let value = self.compile_term(term)?;
                let index = LLVMConstInt(self.types.u64, index as u64, 0);

                let ptr = LLVMBuildGEP2(
                    self.builder,
                    self.types.ptr,
                    env_value,
                    [index].as_mut_ptr(),
                    1,
                    cstr!(),
                );

                LLVMBuildStore(self.builder, value, ptr);
            }

            Ok(env_value)
        }
    }

    fn make_closure(&mut self, env: Vec<(String, Term)>, body: Term) -> Result {
        unsafe {
            let env_value = self.make_closure_environment(env)?;
            let env_len = LLVMConstInt(self.types.u64, self.environment.closure.len() as _, 0);

            let value = self.compile_term(body)?;
            let closure =
                self.make_call("prim__Value_new_closure", &mut [env_value, env_len, value]);

            Ok(closure)
        }
    }
}
