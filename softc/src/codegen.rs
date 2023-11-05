use std::collections::HashMap;

use cranelift::prelude::*;

use cranelift_jit::{JITModule, JITBuilder};
use cranelift_module::{Module, FuncId, DataDescription};
use miette::NamedSource;

use crate::{SrcPos, semantic::SemanticError};
use crate::semantic::Expression;

#[derive(thiserror::Error, miette::Diagnostic, Debug)]
#[diagnostic(url(docsrs))]
#[error("soft error")]
pub struct SoftError {
    #[source_code]
    pub text_source: NamedSource,

    #[related]
    pub related: Vec<InnerError>,
}

#[derive(thiserror::Error, miette::Diagnostic, Debug, Clone)]
#[diagnostic(url(docsrs))]
pub enum InnerError {
    #[error("semantic error: {0}")]
    #[diagnostic(code(soft::semantic))]
    SemanticError(SemanticError),
}

#[derive(Default)]
struct VariableBuilder {
    index: usize,
}

impl VariableBuilder {
    fn new() -> Self {
        Self {
            index: 0,
        }
    }

    fn create_var(&mut self, builder: &mut FunctionBuilder, value: Value) -> Variable {
        let variable = Variable::new(self.index);
        builder.declare_var(variable, types::I64);
        self.index += 1;
        builder.def_var(variable, value);
        variable
    }
}

pub struct Generator {
    pub builder_context: FunctionBuilderContext,
    pub data_description: DataDescription,
    pub location: SrcPos,
    pub ctx: codegen::Context,
    pub module: JITModule,
    variable_builder: VariableBuilder,
}

impl Default for Generator {
    fn default() -> Self {
        let mut flag_builder = settings::builder();

        flag_builder.set("use_colocated_libcalls", "false").unwrap();
        flag_builder.set("is_pic", "false").unwrap();

        let isa_builder = cranelift_native::builder().unwrap_or_else(|msg| {
            panic!("host machine is not supported: {}", msg);
        });

        let isa = isa_builder
            .finish(settings::Flags::new(flag_builder))
            .unwrap();

        let builder = JITBuilder::with_isa(isa, cranelift_module::default_libcall_names());

        let module = JITModule::new(builder);

        Self {
            builder_context: FunctionBuilderContext::new(),
            ctx: module.make_context(),
            data_description: DataDescription::new(),
            location: SrcPos::default(),
            module,
            variable_builder: Default::default(),
        }
    }
}

impl Generator {
    pub fn compile(&mut self) {
        self.module.clear_context(&mut self.ctx);

        self.module.finalize_definitions().unwrap();

        let code = self.module.get_finalized_function(id);

        let code = unsafe { std::mem::transmute::<_, fn() -> i64>(code) };

        let result = code();

        println!("{}", result);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test() {
        let mut generator = Generator::default();
        generator.compile();
    }
}