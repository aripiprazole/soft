use inkwell::attributes::AttributeLoc;
use inkwell::values::BasicValue;
use inkwell::values::BasicValueEnum;
use inkwell::AddressSpace;
use inkwell::IntPredicate;
use itertools::Itertools;

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
            Call(callee, arguments) => self.mk_call(*callee, arguments),
            Prim(_) => todo!(),
        }
    }

    fn mk_call(&mut self, callee: Term, arguments: Vec<Term>) -> BasicValueEnum<'guard> {
        let prev = self.bb.unwrap();
        let parent = prev.get_parent().unwrap();

        let callee = self.term(callee);

        let expected_arity = self.ctx.i8_type().const_int(arguments.len() as u64, false);
        let actual_arity = self.prim__get_function_arity(callee);

        let then = self.ctx.append_basic_block(parent, "call");
        let fail = self.ctx.append_basic_block(parent, "fail");

        // If function.arity = call.arguments
        //   Then proceed to execute the call
        //   Otherwise fails
        let valid = self.builder.build_int_compare(
            IntPredicate::EQ,
            actual_arity.into_int_value(),
            expected_arity,
            "",
        );

        self.builder.build_conditional_branch(valid, then, fail);

        // Fail call
        self.builder.position_at_end(fail);
        self.incorrect_arity_fail(arguments.len() as u8);
        self.builder.build_unreachable();

        // Proceed call
        self.builder.position_at_end(then);

        let ptr = self.prim__get_function_ptr(callee);
        let env = self.prim__get_function_env(callee);

        // Set the first parameter as the closure's context
        let mut params = vec![self.ctx.i64_type().into(); arguments.len()];
        params.insert(0, llvm_type!(self, ptr).into());

        let ty = self.ctx.i64_type().fn_type(&params, false);

        let ptr = self.builder.build_pointer_cast(
            ptr.into_pointer_value(),
            ty.ptr_type(AddressSpace::default()),
            "",
        );

        let mut arguments = arguments
            .into_iter()
            .map(|term| self.term(term).into())
            .collect_vec();
        arguments.insert(0, env.into());

        self.builder
            .build_indirect_call(ty, ptr, &arguments, "")
            .try_as_basic_value()
            .left()
            .expect("The result is not a value")
    }

    fn lambda(&mut self, definition: Definition) -> BasicValueEnum<'guard> {
        let prev = self.bb.unwrap();

        let mut params = vec![self.ctx.i64_type().into(); definition.parameters.len()];
        // Set the first parameter as the closure's context
        params.insert(0, llvm_type!(self, ptr).into());

        let ty = self.ctx.i64_type().fn_type(&params, false);

        let arity = params.len() - 1;
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
        let arity = self.ctx.i8_type().const_int(arity as u64, false);

        self.prim__function(lambda.as_basic_value_enum(), arity.into())
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

    fn incorrect_arity_fail(&mut self, expected: u8) {
        let message = format!("The call failed, because the expected arity is {expected}");
        let message = self.builder.build_global_string_ptr(&message, "call.panic");
        self.soft_panic(message.as_pointer_value().as_basic_value_enum());
    }
}
