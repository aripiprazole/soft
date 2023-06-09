//! This module generates LLVM IR from the [Term] tree.

use inkwell::values::BasicValueEnum;

use crate::specialize::tree::{self, Term};

use super::codegen::CodeGen;

impl<'a> CodeGen<'a> {
    pub fn compile_number(&mut self, number: &tree::Number) -> BasicValueEnum<'a> {
        let value = self
            .llvm_ctx
            .context
            .i64_type()
            .const_int(number.value, false)
            .into();

        self.prim__new_u61(value)
    }

    pub fn compile_term(&mut self, term: &Term) -> BasicValueEnum<'a> {
        match term {
            Term::Number(number) => self.compile_number(number),
            Term::Atom(_) => todo!(),
            Term::TypeOf(_) => todo!(),
            Term::Vector(_) => todo!(),
            Term::Cons(_) => todo!(),
            Term::Nil(_) => todo!(),
            Term::Head(_) => todo!(),
            Term::Tail(_) => todo!(),
            Term::IsNil(_) => todo!(),
            Term::VectorIndex(_) => todo!(),
            Term::VectorLen(_) => todo!(),
            Term::VectorPush(_) => todo!(),
            Term::Box(_) => todo!(),
            Term::Unbox(_) => todo!(),
            Term::CreateClosure(_) => todo!(),
            Term::Binary(_) => todo!(),
            Term::Str(_) => todo!(),
            Term::Bool(_) => todo!(),
            Term::Variable(_) => todo!(),
            Term::Let(_) => todo!(),
            Term::Lambda(_) => todo!(),
            Term::Block(_) => todo!(),
            Term::Quote(_) => todo!(),
            Term::If(_) => todo!(),
            Term::Call(_) => todo!(),
        }
    }
}
