use inkwell::debug_info::{DIBasicType, DICompileUnit, DebugInfoBuilder};
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
}
