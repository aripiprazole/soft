//! The main structure of this module is the [CodeGen] that generates LLVM IR from the AST and a
//! live object of it in order to dispose it in the future.

use std::fmt::Display;
use std::fmt::Formatter;
use std::marker::PhantomData;

use fxhash::FxHashMap;

use inkwell::attributes::AttributeLoc;
use inkwell::basic_block::BasicBlock;
use inkwell::types::FunctionType;
use inkwell::values::AsValueRef;
use inkwell::values::BasicValueEnum;
use inkwell::values::FunctionValue;
use inkwell::OptimizationLevel;

use llvm_sys::core::*;
use llvm_sys::prelude::*;
use llvm_sys::transforms::scalar::*;

use soft_runtime::ptr::FatPtr;
use soft_runtime::ptr::TaggedPtr;

use crate::backend::Runnable;

pub use self::state::{Closed, Open};

/// This module is almost like an enum in type level. It's used to define the state
/// of the function in order to not use a compiled in half function.
mod state {
    pub trait State {}
    pub enum Open {}
    pub enum Closed {}

    impl State for Open {}
    impl State for Closed {}
}

pub type ForeignFunction = unsafe extern "C" fn() -> soft_runtime::ptr::TaggedPtr;

pub struct JitFunction<'a> {
    pub function: inkwell::execution_engine::JitFunction<'a, ForeignFunction>,
    pub engine: inkwell::execution_engine::ExecutionEngine<'a>,
    pub name: String,
}

impl<'a> Runnable for JitFunction<'a> {
    fn run(&self) {
        let res = unsafe { self.function.call() };
        println!("{:?}", FatPtr::from(res));
    }
}

pub trait State: state::State {}

/// A structure that express a compilation of a sequence of soft terms. It's used to generate
/// a [JitFunction].
pub struct Compilable<'a, S: state::State> {
    function: FunctionValue<'a>,
    phantom: PhantomData<S>,
    name: String,
}

impl<'a> Compilable<'a, state::Open> {
    fn new(function: FunctionValue<'a>, name: String) -> Self {
        Self {
            function,
            phantom: PhantomData,
            name,
        }
    }

    fn close(self) -> Compilable<'a, state::Closed> {
        Compilable {
            function: self.function,
            phantom: PhantomData,
            name: self.name,
        }
    }
}

/// Attributes for function generation. They're used inside the LLVM function generation.
#[derive(Clone, Copy)]
pub enum Attribute {
    Cold,
    NoInline,
    NoBuiltIn,
    UwTable,
}

impl Display for Attribute {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Attribute::Cold => write!(f, "cold"),
            Attribute::NoInline => write!(f, "noinline"),
            Attribute::NoBuiltIn => write!(f, "nobuiltin"),
            Attribute::UwTable => write!(f, "uwtable"),
        }
    }
}

/// Code generation options for LLVM.
#[derive(Clone)]
pub struct Options {
    main_attributes: Vec<Attribute>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            main_attributes: vec![
                Attribute::Cold,
                Attribute::NoInline,
                Attribute::NoBuiltIn,
                Attribute::UwTable,
            ],
        }
    }
}

/// Transforms a name into a mangled version of it to generate functions.
fn mangle_name(name: &str) -> String {
    let hash = format!("{:x}", fxhash::hash64(&name));
    let hash = hash[0..8].to_string();
    format!("_S{}{}{hash}", name.len(), name)
}

/// A low level context is used mainly for the LLVM IR generation. It's closer to LLVM than
/// the [FunctionContext] and [CodeGen] structures.
pub struct LowLevelContext<'a> {
    pub context: &'a inkwell::context::Context,
    pub module: inkwell::module::Module<'a>,
    pub builder: inkwell::builder::Builder<'a>,
    pub pass_manager: LLVMPassManagerRef,
}

impl<'a> LowLevelContext<'a> {
    /// Creates a pass manager phase for the module. Introducing a lot of optimizations.
    fn create_pass_manager(module: LLVMModuleRef) -> LLVMPassManagerRef {
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

    /// Creates a new low level context.
    fn new(context: &'a inkwell::context::Context) -> Self {
        let module = context.create_module("soft");
        let pass_manager = Self::create_pass_manager(module.as_mut_ptr());

        Self {
            context,
            module,
            builder: context.create_builder(),
            pass_manager,
        }
    }

    fn create_entry_block(&self, function: FunctionValue<'a>) -> BasicBlock<'a> {
        let entry = self.context.append_basic_block(function, "entry");
        self.builder.position_at_end(entry);
        entry
    }
}

/// This structure is used to keep track of term information during the compilation. It does not
/// relate to how LLVM modules are created but the relationship between [Term] and LLVM.
#[derive(Default)]
pub struct FunctionContext<'a> {
    /// The name stack stuff, every time the program starts, it puts a name in the stack, `soft`,
    /// and every time a `Set` expression is handled, it's added here too.
    ///
    /// It does serves to set names to functions and variables for better debugging in the IR.
    ///
    /// `soft.closure`, etc
    pub name_stack: Vec<String>,

    /// Defines if the file is anonymous
    pub anonymous: Option<String>,

    /// The current function let bound names
    pub names: FxHashMap<String, inkwell::values::BasicValueEnum<'a>>,

    /// The current basic block
    pub basic_block: Option<inkwell::basic_block::BasicBlock<'a>>,
}

impl<'a> FunctionContext<'a> {
    fn push_name(&mut self, name: &str) {
        self.name_stack.push(name.to_string());
    }

    fn basic_block(&self) -> inkwell::basic_block::BasicBlock<'a> {
        self.basic_block.unwrap()
    }

    fn set_basic_block(&mut self, basic_block: inkwell::basic_block::BasicBlock<'a>) {
        self.basic_block = Some(basic_block);
    }
}

/// The main structure of this module is the [CodeGen] that generates LLVM IR from the AST and a
pub struct CodeGen<'a> {
    pub llvm_ctx: LowLevelContext<'a>,
    pub function_ctx: FunctionContext<'a>,
    pub options: &'a Options,
}

impl<'a> CodeGen<'a> {
    pub fn new(context: &'a inkwell::context::Context, options: &'a Options) -> Self {
        Self {
            llvm_ctx: LowLevelContext::new(context),
            function_ctx: FunctionContext::default(),
            options,
        }
    }

    fn add_function(
        &mut self,
        mangled_name: String,
        fun_type: FunctionType<'a>,
    ) -> FunctionValue<'a> {
        self.llvm_ctx
            .module
            .add_function(&mangled_name, fun_type, None)
    }

    fn add_attr(&mut self, fun: FunctionValue<'a>, name: &str) {
        fun.add_attribute(AttributeLoc::Function, self.attr(name));
    }

    /// Initiates a function creation with the given name and type.
    pub fn compiler(&mut self, name: &str, fun_type: FunctionType<'a>) -> Compilable<'a, Open> {
        self.function_ctx.push_name(name);

        let mangled_name = mangle_name(name);

        let function = self.add_function(mangled_name.clone(), fun_type);

        for attr in self.options.main_attributes.clone() {
            self.add_attr(function, &attr.to_string());
        }

        let entry = self.llvm_ctx.create_entry_block(function);
        self.llvm_ctx.builder.position_at_end(entry);
        self.function_ctx.set_basic_block(entry);

        Compilable::new(function, mangled_name)
    }

    pub fn finish(
        &mut self,
        compiler: Compilable<'a, Open>,
        value: &BasicValueEnum<'a>,
    ) -> Compilable<'a, Closed> {
        self.llvm_ctx
            .builder
            .position_at_end(self.function_ctx.basic_block());

        self.llvm_ctx.builder.build_return(Some(value));

        unsafe {
            LLVMRunFunctionPassManager(
                self.llvm_ctx.pass_manager,
                compiler.function.as_value_ref(),
            );
        }

        compiler.close()
    }

    pub fn generate(&mut self, compiler: Compilable<'a, Closed>) -> JitFunction<'a> {
        self.llvm_ctx.module.verify().unwrap_or_else(|err| {
            panic!("Module is broken: {}", err.to_string_lossy());
        });

        let engine = self
            .llvm_ctx
            .module
            .create_jit_execution_engine(OptimizationLevel::Aggressive)
            .expect("Could not create execution engine");

        self.initialize_jit_functions(&engine);
        self.initialize_std_functions();

        unsafe {
            let function = engine
                .get_function::<unsafe extern "C" fn() -> TaggedPtr>(&compiler.name)
                .unwrap_or_else(|_| panic!("Could not find the main function: {0}", compiler.name));

            JitFunction {
                function,
                engine,
                name: compiler.name,
            }
        }
    }
}
