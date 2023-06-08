use inkwell::values::BasicValue;
use inkwell::values::BasicValueEnum;

use crate::specialize::tree::OperationKind;
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
            Operation(kind, operands) if operands.len() == 1 => self.unary(kind, operands),
            Operation(kind, operands) if operands.len() > 1 => self.binary(kind, operands),
            Operation(_, _) => todo!(),
            Call(_, _) => todo!(),
            Prim(_) => todo!(),
        }
    }

    fn binary(&mut self, kind: OperationKind, mut operands: Vec<Term>) -> BasicValueEnum<'guard> {
        use OperationKind::*;
        let operand = operands.remove(0);
        let operand = self.term(operand);

        operands.into_iter().fold(operand, |acc, next| {
            let next = self.term(next);

            match kind {
                Add => self.prim__add_tagged(acc, next),
                Sub => self.prim__sub_tagged(acc, next),
                Mul => self.prim__mul_tagged(acc, next),
                Div => todo!("To divide numbers, it's needed decimal numbers to the result"),
                Mod => self.prim__mod_tagged(acc, next),
                Shl => self.prim__shl_tagged(acc, next),
                Shr => self.prim__shr_tagged(acc, next),
                And => self.prim__and_tagged(acc, next),
                Xor => self.prim__xor_tagged(acc, next),
                Or => self.prim__or_tagged(acc, next),
                Eql => todo!(),
                Neq => todo!(),
                Gtn => todo!(),
                Gte => todo!(),
                Ltn => todo!(),
                Lte => todo!(),
                LAnd => todo!(),
                LOr => todo!(),
                _ => self.binary_will_fail(kind),
            }
        })
    }

    fn unary(&mut self, kind: OperationKind, _operands: Vec<Term>) -> BasicValueEnum<'guard> {
        use OperationKind::*;
        // let operand = operands.first().unwrap();

        match kind {
            Sub => todo!(),
            Not => todo!(),
            _ => self.unary_will_fail(kind),
        }
    }

    fn unary_will_fail(&mut self, kind: OperationKind) -> BasicValueEnum<'guard> {
        let message = format!("The operation {kind} will fail if executed as unary.");
        let message = self
            .builder
            .build_global_string_ptr(&message, "unary.panic");
        self.soft_panic(message.as_pointer_value().as_basic_value_enum());
        self.prim__nil()
    }

    fn binary_will_fail(&mut self, kind: OperationKind) -> BasicValueEnum<'guard> {
        let message = format!("The operation {kind} will fail if executed as binary.");
        let message = self
            .builder
            .build_global_string_ptr(&message, "unary.panic");
        self.soft_panic(message.as_pointer_value().as_basic_value_enum());
        self.prim__nil()
    }
}
