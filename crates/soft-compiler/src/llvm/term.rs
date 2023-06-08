use inkwell::attributes::AttributeLoc;
use inkwell::values::BasicValue;
use inkwell::values::BasicValueEnum;

use crate::specialize::tree::Definition;
use crate::specialize::tree::OperationKind;
use crate::specialize::tree::Term;
use crate::specialize::tree::TermKind::*;

use super::macros::llvm_type;
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
            Bool(true) => todo!(),
            Bool(false) => todo!(),
            Variable(_) => todo!(),
            Let(_, _) => todo!(),
            Set(_, _, _, _) => todo!(),
            Lambda(definition, _) => self.lambda(definition),
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

    fn lambda(&mut self, definition: Definition) -> BasicValueEnum<'guard> {
        let prev = self.bb.unwrap();

        let mut params = vec![self.ctx.i64_type().into(); definition.parameters.len()];
        // Set the first parameter as the closure's context
        params.insert(0, llvm_type!(self, ptr).into());

        let ty = self.ctx.i64_type().fn_type(&params, false);

        let arity = params.len();
        let name_stack = self.name_stack.join(".");
        let local_name = match self.anonymous {
            Some(ref name) => format!("<{}.{name} as soft.function(arity: {arity})>", name_stack),
            None => format!("<{}.local as soft.function(arity: {arity})>", name_stack),
        };
        let lambda = self.module.add_function(&local_name, ty, None);

        lambda.get_nth_param(0).unwrap().set_name("closure_env");

        // Skips the first arguments, since it's the environment argument
        // TODO: handle parameters, add to the ctx
        for (i, param) in lambda.get_params().iter().skip(1).enumerate() {
            // SAFETY: The [`params`] variable has the size [`definition.parameters`]
            let symbol = unsafe { definition.parameters.get_unchecked(i) };

            // Set name of the parameter for debug porpuses
            param.set_name(symbol.name());
        }

        lambda.add_attribute(AttributeLoc::Function, self.attr("nobuiltin"));
        lambda.add_attribute(AttributeLoc::Function, self.attr("uwtable"));
        lambda.add_attribute(AttributeLoc::Param(0), self.attr_value("align", 8));

        let bb = self.ctx.append_basic_block(lambda, "entry");
        self.builder.position_at_end(bb);

        let value = self.term(*definition.body);
        self.builder.build_return(Some(&value));
        self.run_passes(lambda);

        // Return at the old/previous position, before generating code with this definition
        self.builder.position_at_end(prev);

        // Get the function pointer as the new function
        let ty = llvm_type!(self, ptr);
        let lambda = lambda.as_global_value().as_pointer_value();
        let lambda = self.builder.build_pointer_cast(lambda, ty, "");

        self.prim__function(lambda.as_basic_value_enum())
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
