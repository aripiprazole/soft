use inkwell::values::BasicValueEnum;

use crate::specialize::tree::Term;
use crate::specialize::tree::TermKind::*;

use super::Codegen;

impl<'guard> Codegen<'guard> {
    pub fn term(&mut self, term: Term) -> BasicValueEnum<'guard> {
        match term.data {
            Atom(_) => todo!(),
            Number(_) => self.ctx.i64_type().const_int(10, false).into(),
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
