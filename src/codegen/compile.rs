use im::HashMap;

use crate::specialized::Term;
use llvm_sys::LLVMIntPredicate::LLVMIntEQ;

use super::*;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum CompileError {
    UndefinedEnvRef(String),
    UndefinedLocalRef(String),
    UndefinedGlobalRef(String),
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
            Lam(_, args, box body) => {
                let old_block = LLVMGetInsertBlock(self.builder);

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

                let body = self.compile_term(body)?;
                LLVMBuildRet(self.builder, body);

                self.pop_scope();
                self.current_fn = current;
                LLVMPositionBuilderAtEnd(self.builder, old_block);

                let arity = LLVMConstInt(self.types.u64, args.len() as _, 0);
                let new_fn = LLVMBuildPointerCast(
                    self.builder,
                    new_fn,
                    LLVMPointerType(self.types.ptr, 0),
                    cstr!(),
                );

                Ok(self.call_prim("prim__Value_function", &mut [arity, new_fn]))
            }
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
            App(box callee, args) => {
                let is_fun = LLVMAppendBasicBlockInContext(self.context, current, cstr!());
                let else_br = LLVMAppendBasicBlockInContext(self.context, current, cstr!());
                let is_closure = LLVMAppendBasicBlockInContext(self.context, current, cstr!());
                let else_panic = LLVMAppendBasicBlockInContext(self.context, current, cstr!());

                let next = LLVMAppendBasicBlockInContext(self.context, current, cstr!());

                let args_len = LLVMConstInt(self.types.u64, args.len() as _, 0);

                let callee_value = self.compile_term(callee)?;

                let address = self.call_prim("prim__check_arity", &mut [callee_value, args_len]);
                let is_true = self.call_prim("prim__is_null", &mut [address]);

                LLVMBuildCondBr(self.builder, is_true, else_br, is_fun);

                LLVMPositionBuilderAtEnd(self.builder, is_fun);

                let mut args_t = vec![self.types.ptr; args.len()];
                let call_t =
                    LLVMFunctionType(self.types.ptr, args_t.as_mut_ptr(), args.len() as _, 0);

                let fn_ptr = LLVMBuildPointerCast(
                    self.builder,
                    address,
                    LLVMPointerType(call_t, 0),
                    cstr!(),
                );

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

                let fun = self.call_prim("prim__closure_get_fn", &mut [callee_value]);
                let is_true = self.call_prim("prim__is_null", &mut [fun]);
                LLVMBuildCondBr(self.builder, is_true, else_panic, is_closure);

                LLVMPositionBuilderAtEnd(self.builder, is_closure);

                let args_len = LLVMConstInt(self.types.u64, (args.len() + 1) as _, 0);
                let address = self.call_prim("prim__check_arity", &mut [fun, args_len]);

                let env = self.call_prim("prim__closure_get_env", &mut [callee_value]);

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

                let fn_ptr = LLVMBuildPointerCast(
                    self.builder,
                    address,
                    LLVMPointerType(call_t, 0),
                    cstr!(),
                );

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
            Closure(env, box body) => {
                self.enter_scope();

                let closure_environment = env
                    .iter()
                    .enumerate()
                    .map(|(index, (name, _))| (name.clone(), index))
                    .collect();

                self.environment.closure = closure_environment;

                let closure_environment = self.build_closure_environment(env)?;
                let closure = self.build_closure(closure_environment, body)?;

                self.pop_scope();

                Ok(closure)
            }
            EnvRef(sym) => {
                let index = self
                    .environment
                    .closure
                    .get(&sym)
                    .ok_or_else(|| CompileError::UndefinedEnvRef(sym.clone()))?;

                let env_param = LLVMGetLastParam(current);

                let index_value = LLVMConstInt(self.types.u64, *index as _, 0);
                let value = self.call_prim("prim__Value_gep", &mut [env_param, index_value]);

                Ok(value)
            }
            LocalRef(sym) => {
                let symbol = self
                    .environment
                    .symbols
                    .get(&sym)
                    .ok_or_else(|| CompileError::UndefinedLocalRef(sym))?;

                let value = LLVMBuildLoad2(self.builder, symbol.value_type, symbol.value, cstr!());
                Ok(value)
            }
            GlobalRef(sym) => {
                let sym_lit = LLVMBuildGlobalStringPtr(self.builder, cstr!(sym), cstr!());

                let value = self.call_prim("prim__global_get", &mut [self.global_sym, sym_lit]);

                Ok(value)
            }
            Set(name, _, box value_term) => {
                let sym = LLVMBuildGlobalStringPtr(self.builder, cstr!(name), cstr!());
                let value = self.compile_term(value_term)?;

                self.call_prim("prim__global_set", &mut [self.global_sym, sym, value]);

                self.compile_term(Term::Nil)
            }
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

    unsafe fn build_closure_environment(
        &mut self,
        env: Vec<(String, Term)>,
    ) -> Result<LLVMValueRef, CompileError> {
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

    unsafe fn build_closure(
        &mut self,
        env: LLVMValueRef,
        body: Term,
    ) -> Result<LLVMValueRef, CompileError> {
        let env_len = LLVMConstInt(self.types.u64, self.environment.closure.len() as _, 0);

        let value = self.compile_term(body)?;
        let closure = self.call_prim("prim__Value_new_closure", &mut [env, env_len, value]);

        Ok(closure)
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

#[derive(Debug, PartialEq, Eq, Clone)]
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

type FunctionRef<'a> = (&'a str, *mut libc::c_void);

impl Codegen {
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
}

impl Environment {
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
