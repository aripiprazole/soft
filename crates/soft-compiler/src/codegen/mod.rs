use std::sync::Arc;

use crate::specialize::tree::Term;
use cranelift::prelude::{isa::TargetIsa, *};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module};
use fxhash::FxHashMap;

pub struct Codegen<'src> {
    /// Holds all the names used in the current context. To be easier to split code in further steps
    ///
    /// TODO
    pub names: FxHashMap<String, Term<'src>>,

    pub module: JITModule,
    pub builder_context: FunctionBuilderContext,
    pub ctx: codegen::Context,
}

pub struct DeclContext<'guard> {
    u64: types::Type,
    builder: FunctionBuilder<'guard>,
}

impl Default for Codegen<'_> {
    fn default() -> Self {
        let isa = default_flags();
        let builder = JITBuilder::with_isa(isa, cranelift_module::default_libcall_names());
        let module = JITModule::new(builder);

        Self {
            names: Default::default(),
            builder_context: FunctionBuilderContext::new(),
            ctx: codegen::Context::new(),
            module,
        }
    }
}

pub fn default_flags() -> Arc<dyn TargetIsa> {
    let mut flag_builder = settings::builder();
    flag_builder.set("use_colocated_libcalls", "false").unwrap();
    flag_builder.set("is_pic", "false").unwrap();
    let isa_builder = cranelift_native::builder().unwrap_or_else(|msg| {
        panic!("host machine is not supported: {msg}");
    });
    isa_builder
        .finish(settings::Flags::new(flag_builder))
        .unwrap()
}

impl Codegen<'_> {
    pub fn main_function(&mut self, expr: Term) -> Result<*const u8, String> {
        let u64_type = self.module.target_config().pointer_type();

        // Our toy language currently only supports one return value, though
        // Cranelift is designed to support more.
        self.ctx
            .func
            .signature
            .returns
            .push(AbiParam::new(u64_type));

        // Create the builder to build a function.
        let builder = FunctionBuilder::new(&mut self.ctx.func, &mut self.builder_context);

        let mut decl = DeclContext {
            builder,
            u64: u64_type,
        };

        // Create the entry block, to start emitting code in.
        let entry = decl.builder.create_block();

        decl.builder.append_block_params_for_function_params(entry);
        decl.builder.switch_to_block(entry);
        decl.builder.seal_block(entry);

        let return_value = Self::codegen(&mut decl, expr);

        decl.builder.ins().return_(&[return_value]);

        if let Err(errors) = self.ctx.verify(default_flags().as_ref()) {
            println!("{errors}");
            panic!("failed to verify the function generated")
        }

        // Next, declare the function to jit. Functions must be declared
        // before they can be called, or defined.
        //
        // TODO: This may be an area where the API should be streamlined; should
        // we have a version of `declare_function` that automatically declares
        // the function?
        let id = self
            .module
            .declare_function("main", Linkage::Export, &self.ctx.func.signature)
            .map_err(|err| err.to_string())?;

        self.module
            .define_function(id, &mut self.ctx)
            .map_err(|e| e.to_string())?;

        // Now that compilation is finished, we can clear out the context state.
        self.module.clear_context(&mut self.ctx);

        // Finalize the functions which we just defined, which resolves any
        // outstanding relocations (patching in addresses, now that they're
        // available).
        self.module.finalize_definitions().unwrap();

        // We can now retrieve a pointer to the machine code.
        let code = self.module.get_finalized_function(id);

        Ok(code)
    }

    pub fn codegen(decl: &mut DeclContext, expr: Term) -> Value {
        use crate::specialize::tree::TermKind::*;

        match expr.data {
            Atom(_) => todo!(),
            Number(value) => decl.builder.ins().iconst(decl.u64, value as i64),
            String(_) => todo!(),
            Bool(_) => todo!(),
            Variable(_) => todo!(),
            Let(_, _) => todo!(),
            Set(_, _, _, _) => todo!(),
            Lambda(_, _) => todo!(),
            Block(_) => todo!(),
            Quote(_) => todo!(),
            If(_, _, _) => todo!(),
            Operation(_, _) => todo!(),
            Call(_, _) => todo!(),
            Prim(_) => todo!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        location::{Loc, Spanned},
        specialize::tree::TermKind,
    };

    use super::*;

    #[test]
    fn it_works() {
        let mut codegen = Codegen::default();
        let expr = TermKind::Number(10);

        unsafe {
            let f = codegen
                .main_function(Spanned::new(Loc(0)..Loc(0), expr))
                .unwrap();
            let f = std::mem::transmute::<_, unsafe fn() -> u64>(f);

            println!("f() = {:?}", f());
        }
    }
}
