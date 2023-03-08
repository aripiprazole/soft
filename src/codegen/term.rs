use super::{
    compile::Result,
    helpers::{function_type, pointer_type, IRContext, IRModule},
    *,
};
use crate::{
    codegen::{
        compile::{CompileError, SymbolRef},
        helpers::IRBuilder,
    },
    specialized::Term,
};

impl Codegen {
    pub fn compile_term(&mut self, term: Term) -> Result {
        use Term::*;

        let current_fn = self.current_fn;

        match term {
            Quote(_) => todo!(),
            Nil => Ok(self.make_call("prim__Value_nil", &mut [])),
            Num(n) => {
                let x = self.make_int_const(n);
                Ok(self.make_call("prim__Value_new_num", &mut [x]))
            }
            Cons(box head, box tail) => {
                let head = self.compile_term(head)?;
                let tail = self.compile_term(tail)?;

                Ok(self.make_call("prim__Value_cons", &mut [head, tail]))
            }
            Set(sym, _, box value_term) => {
                let sym = self.builder.build_global_string_ptr(&sym, "");
                let value = self.compile_term(value_term)?;

                self.make_call("prim__global_set", &mut [self.global_sym, sym, value]);
                self.compile_term(Term::Nil)
            }
            GlobalRef(sym) => {
                let sym = self.builder.build_global_string_ptr(&sym, "");
                let value = self.make_call("prim__global_get", &mut [self.global_sym, sym]);

                Ok(value)
            }
            LocalRef(sym) => {
                let symbol = self
                    .environment
                    .symbols
                    .get(&sym)
                    .ok_or(CompileError::UndefinedLocalRef(sym))?;

                let value = self.builder.build_load(symbol.kind, symbol.value, "");

                Ok(value)
            }
            EnvRef(sym) => unsafe {
                let index = self
                    .environment
                    .closure
                    .get(&sym)
                    .ok_or_else(|| CompileError::UndefinedEnvRef(sym.clone()))?;

                let env_param = LLVMGetLastParam(current_fn);

                let index_value = self.make_int_const(*index as _);
                let value = self.make_call("prim__Value_gep", &mut [env_param, index_value]);

                Ok(value)
            },
            Closure(env, box term) => self.compile_closure(env, term),
            Let(entries, box term) => self.compile_let(entries, term),
            If(box cond, box then, box otherwise) => self.compile_if(cond, then, otherwise),
            Lam(_, args, box term) => self.compile_lam(args, term),
            App(box callee, args) => self.compile_app(callee, args),
        }
    }

    fn compile_closure(&mut self, env: Vec<(String, Term)>, term: Term) -> Result {
        self.enter_scope();

        self.environment.closure = env
            .iter()
            .enumerate()
            .map(|(index, (name, _))| (name.clone(), index))
            .collect();

        let closure = self.make_closure(env, term)?;

        self.pop_scope();

        Ok(closure)
    }

    fn compile_let(&mut self, entries: Vec<(String, Term)>, term: Term) -> Result {
        self.enter_scope();

        for (name, value) in entries {
            let value = self.compile_term(value)?;
            let alloca = self.builder.build_alloca(self.types.ptr, &name);
            self.builder.build_store(value, alloca);

            let symbol_ref = SymbolRef::new(self.types.ptr, alloca);
            self.environment.symbols.insert(name, symbol_ref);
        }

        let value = self.compile_term(term)?;

        self.pop_scope();

        Ok(value)
    }

    fn compile_if(&mut self, cond: Term, then: Term, otherwise: Term) -> Result {
        let current_fn = self.current_fn;

        let next_br = self.context.append_basic_block(current_fn, "");
        let then_br = self.context.append_basic_block(current_fn, "");
        let otherwise_br = self.context.append_basic_block(current_fn, "");

        let cond_value = self.compile_term(cond)?;
        let cond_value = self.make_if(cond_value);

        self.builder
            .build_cond_br(cond_value, then_br, otherwise_br);
        self.builder.position_at_end(then_br);

        let then_value = self.compile_term(then)?;
        self.builder.build_br(next_br);
        self.builder.position_at_end(otherwise_br);

        let otherwise_value = self.compile_term(otherwise)?;
        self.builder.build_br(next_br);
        self.builder.position_at_end(next_br);

        unsafe {
            let value = LLVMBuildPhi(self.builder, self.types.ptr, cstr!());

            LLVMAddIncoming(
                value,
                [then_value, otherwise_value].as_mut_ptr(),
                [then_br, otherwise_br].as_mut_ptr(),
                2,
            );

            Ok(value)
        }
    }

    fn compile_lam(&mut self, args: Vec<String>, term: Term) -> Result {
        let current_fn = self.current_fn;
        let current_block = self.builder.insertion_block();

        self.enter_scope();

        let mut arg_types = vec![self.types.ptr; args.len()];
        let new_fn = self
            .module
            .add_function("", arg_types.as_mut_slice(), self.types.ptr);

        self.current_fn = new_fn;

        let entry = self.context.append_basic_block(new_fn, "entry");
        self.builder.position_at_end(entry);

        for (index, name) in args.iter().enumerate() {
            let alloca = self.builder.build_alloca(self.types.ptr, "");
            let arg_value = unsafe { LLVMGetParam(new_fn, index as _) };
            self.builder.build_store(arg_value, alloca);

            let symbol_ref = SymbolRef::new(self.types.ptr, alloca);
            self.environment.symbols.insert(name.clone(), symbol_ref);
        }

        let value = self.compile_term(term)?;
        self.builder.build_ret(value);
        self.pop_scope();
        self.current_fn = current_fn;
        self.builder.position_at_end(current_block);

        let arity = self.make_int_const(args.len() as _);
        let new_fn = self
            .builder
            .build_pointer_cast(new_fn, pointer_type!(self.types.ptr), "");

        Ok(self.make_call("prim__Value_function", &mut [arity, new_fn]))
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
        let current_fn = self.current_fn;

        let is_fun = self.context.append_basic_block(current_fn, "");
        let else_br = self.context.append_basic_block(current_fn, "");
        let is_closure = self.context.append_basic_block(current_fn, "");
        let else_panic = self.context.append_basic_block(current_fn, "");

        let next = self.context.append_basic_block(current_fn, "");

        let args_len = self.make_int_const(args.len() as _);

        let callee_value = self.compile_term(callee)?;

        let address = self.make_call("prim__check_arity", &mut [callee_value, args_len]);
        let is_true = self.make_call("prim__is_null", &mut [address]);

        self.builder.build_cond_br(is_true, else_br, is_fun);
        self.builder.position_at_end(is_fun);

        let fn_type = function_type!(self.types.ptr, self.types.ptr; args.len());

        let fn_ptr = self
            .builder
            .build_pointer_cast(address, pointer_type!(fn_type), "");

        let mut args_value = args
            .iter()
            .map(|n| self.compile_term(n.clone()))
            .collect::<Result<Vec<_>>>()?;

        let res2 = self
            .builder
            .build_call(fn_type, fn_ptr, args_value.as_mut_slice(), "");

        self.builder.build_br(next);
        self.builder.position_at_end(else_br);

        let fun = self.make_call("prim__closure_get_fn", &mut [callee_value]);
        let is_true = self.make_call("prim__is_null", &mut [fun]);
        self.builder.build_cond_br(is_true, else_panic, is_closure);

        self.builder.position_at_end(is_closure);

        let args_len = self.make_int_const((args.len() + 1) as _);
        let address = self.make_call("prim__check_arity", &mut [fun, args_len]);

        let env = self.make_call("prim__closure_get_env", &mut [callee_value]);

        let mut args_value = args
            .iter()
            .map(|n| self.compile_term(n.clone()))
            .collect::<Result<Vec<_>>>()?;

        args_value.push(env);

        let fn_type = function_type!(self.types.ptr, self.types.ptr; args_value.len());

        let fn_ptr = self
            .builder
            .build_pointer_cast(address, pointer_type!(fn_type), "");

        let res1 = self
            .builder
            .build_call(fn_type, fn_ptr, args_value.as_mut_slice(), "");

        self.builder.build_br(next);
        self.builder.position_at_end(else_panic);
        self.builder.build_unreachable();
        self.builder.position_at_end(next);

        unsafe {
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
        let env_len = self.make_int_const(env.len() as _);

        let env_value = self.builder.build_array_alloca(self.types.ptr, env_len, "");

        for (index, (_, term)) in env.into_iter().enumerate() {
            let value = self.compile_term(term)?;
            let index = self.make_int_const(index as u64);

            let ptr = self
                .builder
                .build_gep(self.types.ptr, env_value, &mut [index], "");

            self.builder.build_store(value, ptr);
        }

        Ok(env_value)
    }

    fn make_closure(&mut self, env: Vec<(String, Term)>, body: Term) -> Result {
        let env_value = self.make_closure_environment(env)?;
        let env_len = self.make_int_const(self.environment.closure.len() as _);

        let value = self.compile_term(body)?;
        let closure = self.make_call("prim__closure", &mut [env_value, env_len, value]);

        Ok(closure)
    }
}
