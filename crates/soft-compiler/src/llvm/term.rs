use inkwell::values::BasicValueEnum;

use crate::specialize::tree::Term;
use crate::specialize::tree::TermKind::*;

use super::Codegen;

impl<'guard> Codegen<'guard> {
    pub fn term(&mut self, term: Term) -> BasicValueEnum<'guard> {
        match term.data {
            Atom(_) => todo!(),
            Number(value) => {
                let value = self.ctx.i64_type().const_int(value, false).into();

                self.prim__new_u61(value)
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
