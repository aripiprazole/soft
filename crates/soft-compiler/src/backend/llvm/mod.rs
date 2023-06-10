//! This module creates a backend using the [CodeGen] structure.

use crate::specialize::tree::Term;

use self::codegen::{CodeGen, JitFunction};

use super::Backend;

pub mod codegen;
pub mod macros;
pub mod runtime;
pub mod term;

pub struct Context<'a> {
    ctx: inkwell::context::Context,
    engines: Vec<inkwell::execution_engine::ExecutionEngine<'a>>,
    config: &'a codegen::Options,
}

impl<'a> Backend<'a> for Context<'a> {
    type Object = JitFunction<'a>;
    type Config = codegen::Options;

    fn compile(&'a self, terms: Vec<Term<'a>>) -> super::Result<Self::Object> {
        let mut code_gen = CodeGen::new(&self.ctx, self.config);

        code_gen.initialize_std_functions();
        code_gen.setup_attributes();

        let fun_type = self.ctx.i64_type().fn_type(&[], false);
        let fun = code_gen.compiler("main", fun_type);

        let mut ret = None;

        for term in terms {
            ret = Some(code_gen.compile_term(&term));
        }

        let fun = code_gen.finish(fun, &ret.unwrap());
        let fun = code_gen.generate(fun);

        Ok(fun)
    }

    fn new(config: &'a Self::Config) -> Self {
        Self {
            ctx: inkwell::context::Context::create(),
            config,
            engines: vec![],
        }
    }
}
