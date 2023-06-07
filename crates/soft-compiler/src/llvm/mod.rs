use fxhash::FxHashMap;
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::debug_info::{AsDIScope, DWARFEmissionKind, DWARFSourceLanguage};
use inkwell::module::Module;
use inkwell::values::{AsValueRef, BasicValueEnum, FunctionValue};
use llvm_sys::core::*;
use llvm_sys::debuginfo::LLVMDIFlagPrototyped;
use llvm_sys::prelude::*;
use llvm_sys::transforms::instcombine::LLVMAddInstructionCombiningPass;
use llvm_sys::transforms::scalar::*;

use crate::specialize::tree::Term;

use self::di::DIContext;

pub mod di;
pub(crate) mod macros;
pub mod runtime;
pub mod term;

pub struct Codegen<'guard> {
    pub ctx: &'guard Context,
    pub module: Module<'guard>,
    pub builder: Builder<'guard>,
    pub fpm: LLVMPassManagerRef,

    //>>>Contextual stuff
    pub di: DIContext<'guard>,

    /// The current function let bound names
    pub names: FxHashMap<String, BasicValueEnum<'guard>>,

    /// The current basic block
    pub bb: Option<inkwell::basic_block::BasicBlock<'guard>>,
    //<<<
}

impl<'guard> Codegen<'guard> {
    pub fn new(ctx: &'guard Context) -> Self {
        let module = ctx.create_module("SOFT");
        let fpm = Self::create_fpm(module.as_mut_ptr());

        let (dibuilder, dicu) = module.create_debug_info_builder(
            true,
            /* language */ DWARFSourceLanguage::C,
            /* filename */ "awa.soft",
            /* directory */ ".",
            /* producer */ "Soft",
            /* is_optimized */ false,
            /* compiler command line flags */ "",
            /* runtime_ver */ 0,
            /* split_name */ "",
            /* kind */ DWARFEmissionKind::Full,
            /* dwo_id */ 0,
            /* split_debug_inling */ false,
            /* debug_info_for_profiling */ false,
            "/",
            "/",
        );

        Codegen {
            ctx,
            fpm,
            di: DIContext::new(dibuilder, dicu),
            module,
            builder: ctx.create_builder(),
            names: Default::default(),
            bb: None,
        }
    }

    pub fn main(&mut self, name: &str, term: Term) -> String {
        let fun_type = self.ctx.i64_type().fn_type(&[], false);
        let name = self.create_name(name);
        let fun = self.module.add_function(&name, fun_type, None);

        let difile = self.di.builder.create_file("main.soft", "src");
        let difunction = self.di.create_function_type(0, difile);
        let difunction = self.di.builder.create_function(
            /* scope */ self.di.cu.as_debug_info_scope(),
            /* func name */ "main",
            /* linkage_name */ None,
            /* file */ self.di.cu.get_file(),
            /* line_no */ 10,
            /* DIType */ difunction,
            /* is_local_to_unit */ false,
            /* is_definition */ true,
            /* scope_line */ 10,
            /* flags */ LLVMDIFlagPrototyped,
            /* is_optimized */ false,
        );
        fun.set_subprogram(difunction);

        let entry = self.ctx.append_basic_block(fun, "entry");
        self.builder.position_at_end(entry);
        self.bb = Some(entry);

        let location = self.di.builder.create_debug_location(
            self.ctx,
            0,
            10,
            difunction.as_debug_info_scope(),
            None,
        );
        self.builder.set_current_debug_location(location);

        let value = self.term(term);
        self.builder.build_return(Some(&value));
        self.run_passes(fun);

        name
    }

    fn run_passes(&self, f: FunctionValue) {
        unsafe {
            LLVMRunFunctionPassManager(self.fpm, f.as_value_ref());
        }
    }

    fn create_fpm(module: LLVMModuleRef) -> LLVMPassManagerRef {
        unsafe {
            let fpm = LLVMCreateFunctionPassManagerForModule(module);
            LLVMAddInstructionCombiningPass(fpm);
            LLVMAddReassociatePass(fpm);
            LLVMAddGVNPass(fpm);
            LLVMAddCFGSimplificationPass(fpm);
            LLVMInitializeFunctionPassManager(fpm);
            fpm
        }
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
    use soft_runtime::ptr::TaggedPtr;

    use crate::{
        parser::parse,
        specialize::{closure::ClosureConvert, specialize},
    };

    use super::Codegen;

    #[test]
    fn it_works() {
        let context = Context::create();
        let mut codegen = Codegen::new(&context);
        codegen.initialize_std_functions();

        let code = parse("(lambda () 42)").unwrap();
        let mut code = specialize(code.first().unwrap().clone());
        code.closure_convert();

        let main = codegen.main("main", code);

        codegen.di.builder.finalize();

        println!("{}", codegen.module.print_to_string().to_string_lossy());

        // Verify the LLVM module integrity
        codegen.module.verify().unwrap_or_else(|err| {
            panic!("Module is broken: {}", err.to_string_lossy());
        });

        let engine = codegen
            .module
            .create_jit_execution_engine(OptimizationLevel::Aggressive)
            .expect("Could not create execution engine");

        codegen.initialize_jit_functions(&engine);

        unsafe {
            let f = engine
                .get_function::<unsafe extern "C" fn() -> TaggedPtr>(&main)
                .unwrap_or_else(|_| panic!("Could not find the main function: {main}"));

            println!("f.call() = {:?}", f.call().tag());
        }
    }
}
