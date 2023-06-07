use fxhash::FxHashMap;
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::values::BasicValueEnum;

use crate::specialize::tree::Term;

pub(crate) mod macros;
pub mod runtime;
pub mod term;

pub struct Codegen<'guard> {
    pub ctx: &'guard Context,
    pub module: Module<'guard>,
    pub builder: Builder<'guard>,

    //>>>Contextual stuff
    /// The current function let bound names
    pub names: FxHashMap<String, BasicValueEnum<'guard>>,

    /// The current basic block
    pub bb: Option<inkwell::basic_block::BasicBlock<'guard>>,
    //<<<
}

impl<'guard> Codegen<'guard> {
    pub fn new(ctx: &'guard Context) -> Self {
        Codegen {
            ctx,
            module: ctx.create_module("SOFT"),
            builder: ctx.create_builder(),
            names: Default::default(),
            bb: None,
        }
    }

    pub fn main(&mut self, name: &str, term: Term) -> String {
        let fun_type = self.ctx.i64_type().fn_type(&[], false);
        let name = self.create_name(name);
        let fun = self.module.add_function(&name, fun_type, None);

        let entry = self.ctx.append_basic_block(fun, "entry");
        self.builder.position_at_end(entry);
        self.bb = Some(entry);

        let value = self.term(term);
        self.builder.build_return(Some(&value));

        name
    }

    fn create_name(&mut self, name: &str) -> String {
        let hash = format!("{:x}", fxhash::hash64(&name));
        let hash = hash[0..8].to_string();
        format!("_S{}{}{hash}", name.len(), name)
    }
}

#[cfg(test)]
mod tests {
    use inkwell::{context::Context, OptimizationLevel};
    use soft_runtime::internal::*;
    use soft_runtime::ptr::TaggedPtr;

    use crate::specialize::tree::TermKind;

    use super::macros;
    use super::Codegen;

    #[test]
    fn it_works() {
        use crate::specialize::tree::OperationKind::*;

        let context = Context::create();
        let mut codegen = Codegen::new(&context);
        codegen.initialize_std_functions();
        let main = codegen.main(
            "main",
            TermKind::Operation(
                Add,
                vec![TermKind::Number(10).into(), TermKind::Number(30).into()],
            )
            .into(),
        );

        println!("{}", codegen.module.print_to_string().to_string_lossy());

        // Verify the LLVM module integrity
        codegen.module.verify().unwrap_or_else(|err| {
            panic!("Module is broken: {}", err.to_string_lossy());
        });

        let engine = codegen
            .module
            .create_jit_execution_engine(OptimizationLevel::None)
            .expect("Could not create execution engine");

        macros::register_jit_function!(
            codegen,
            engine,
            [
                prim__new_u61,
                prim__add_tagged,
                prim__sub_tagged,
                prim__mul_tagged,
                prim__mod_tagged,
                prim__shl_tagged,
                prim__shr_tagged,
                prim__and_tagged,
                prim__xor_tagged,
                prim__or_tagged,
            ]
        );

        unsafe {
            let f = engine
                .get_function::<unsafe extern "C" fn() -> TaggedPtr>(&main)
                .unwrap_or_else(|_| panic!("Could not find the main function: {main}"));

            println!("f.call() = {}", f.call().assert().number());
        }
    }
}
