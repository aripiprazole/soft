use fxhash::FxHashMap;
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::values::BasicValueEnum;

pub(crate) mod macros;
pub mod runtime;

pub struct Codegen<'guard> {
    pub context: &'guard Context,
    pub module: Module<'guard>,
    pub builder: Builder<'guard>,

    //>>>Contextual stuff
    /// The current function let bound names
    pub names: FxHashMap<String, BasicValueEnum<'guard>>,

    /// The context parameter for the apply function
    pub ctx: Option<inkwell::values::BasicValueEnum<'guard>>,

    /// The current basic block
    pub bb: Option<inkwell::basic_block::BasicBlock<'guard>>,
    //<<<
}

impl<'guard> Codegen<'guard> {
    pub fn new(context: &'guard Context) -> Codegen<'guard> {
        Codegen {
            context,
            module: context.create_module("SOFT"),
            builder: context.create_builder(),
            names: Default::default(),
            ctx: None,
            bb: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use inkwell::context::Context;

    use super::Codegen;

    #[test]
    fn it_works() {
        let context = Context::create();
        let codegen = Codegen::new(&context);
        codegen.initialize_std_functions();
    }
}
