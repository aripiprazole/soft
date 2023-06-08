use inkwell::debug_info::{DIBasicType, DICompileUnit, DIFile, DISubroutineType, DebugInfoBuilder};
use llvm_sys::debuginfo::LLVMDIFlagPublic;

pub struct DIContext<'guard> {
    pub builder: DebugInfoBuilder<'guard>,
    pub cu: DICompileUnit<'guard>,
    pub u64: DIBasicType<'guard>,
}

impl<'guard> DIContext<'guard> {
    pub fn new(builder: DebugInfoBuilder<'guard>, cu: DICompileUnit<'guard>) -> Self {
        let u64 = builder
            .create_basic_type("u64", 64, 0, LLVMDIFlagPublic)
            .unwrap();

        Self { builder, cu, u64 }
    }

    pub(crate) fn create_function_type(
        &self,
        arity: usize,
        unit: DIFile<'guard>,
    ) -> DISubroutineType {
        let return_type = Some(self.u64.as_type());
        let mut params = vec![];
        for _ in 0..arity {
            params.push(self.u64.as_type());
        }

        self.builder
            .create_subroutine_type(unit, return_type, &params, 0)
    }
}
