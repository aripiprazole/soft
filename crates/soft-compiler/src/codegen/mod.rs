use std::sync::Arc;

use crate::specialize::tree::Term;
use cranelift::prelude::{isa::TargetIsa, *};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module};
use fxhash::FxHashMap;
use soft_runtime::FatPtr;

pub struct Codegen<'src> {
    /// Holds all the names used in the current context. To be easier to split code in further steps
    ///
    /// TODO
    pub names: FxHashMap<String, Term<'src>>,

    pub module: JITModule,
    pub builder_ctx: FunctionBuilderContext,
    pub ctx: codegen::Context,
}

pub struct DeclContext<'guard> {
    u64: types::Type,
    module: &'guard JITModule,
    builder: FunctionBuilder<'guard>,
}

impl Default for Codegen<'_> {
    fn default() -> Self {
        let isa = default_flags();
        let builder = JITBuilder::with_isa(isa, cranelift_module::default_libcall_names());
        let module = JITModule::new(builder);

        Self {
            names: Default::default(),
            builder_ctx: FunctionBuilderContext::new(),
            ctx: codegen::Context::new(),
            module,
        }
    }
}

impl Codegen<'_> {
    pub fn main(&mut self, expr: Term) -> Result<unsafe fn() -> FatPtr, String> {
        let mut decl = DeclContext::new(self);
        decl.register(stringify!(new_u64), &[decl.u64], decl.u64);

        // Create the entry block, to start emitting code in.
        let entry = decl.builder.create_block();

        decl.builder.append_block_params_for_function_params(entry);
        decl.builder.switch_to_block(entry);
        decl.builder.seal_block(entry);

        let return_value = decl.codegen(expr);
        decl.builder.ins().return_(&[return_value]);

        if let Err(errors) = self.ctx.verify(default_flags().as_ref()) {
            println!("{errors}");
            panic!("failed to verify the function generated")
        }

        // Next, declare the function to jit. Functions must be declared
        // before they can be called, or defined.
        let id = self
            .module
            .declare_function("main", Linkage::Export, &self.ctx.func.signature)
            .map_err(|err| err.to_string())?;

        self.module
            .define_function(id, &mut self.ctx)
            .map_err(|e| e.to_string())?;

        // Now that compilation is finished, we can clear out the context state.

        self.module.clear_context(&mut self.ctx);
        self.module.finalize_definitions().unwrap();
        let code = self.module.get_finalized_function(id);

        Ok(unsafe { std::mem::transmute(code) })
    }
}

impl<'guard> DeclContext<'guard> {
    pub fn new(cg: &'guard mut Codegen) -> Self {
        // u64 = pointer size type
        let u64_type = cg.module.target_config().pointer_type();

        cg.ctx.func.signature.returns.push(AbiParam::new(u64_type));

        // Create the builder to build a function.
        let builder = FunctionBuilder::new(&mut cg.ctx.func, &mut cg.builder_ctx);

        Self {
            builder,
            module: &cg.module,
            u64: u64_type,
        }
    }

    pub fn register(
        &mut self,
        name: &str,
        parameters: &[types::Type],
        return_type: types::Type,
    ) -> Signature {
        let mut sig = self.module.make_signature();
        for param in parameters {
            sig.params.push(AbiParam::new(*param));
        }
        sig.returns.push(AbiParam::new(return_type));
        sig
    }

    pub fn call(&mut self, name: &str, args: &[Value]) -> Value {
        todo!()
    }

    pub fn codegen(&mut self, expr: Term) -> Value {
        use crate::specialize::tree::TermKind::*;

        match expr.data {
            Atom(_) => todo!(),
            Number(value) => {
                let value = self.builder.ins().iconst(self.u64, value as i64);

                self.call("new_u64", &[value])
            }
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

#[cfg(test)]
mod tests {
    use crate::specialize::tree::TermKind;

    use super::*;

    #[test]
    fn it_works() {
        let mut codegen = Codegen::default();
        let expr = TermKind::Number(10);

        unsafe {
            let f = codegen.main(expr.into()).unwrap();

            println!("f() = {:?}", f());
        }
    }
}
