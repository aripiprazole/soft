//! This module generates LLVM IR from the [Term] tree.

use itertools::Itertools;

use fxhash::FxHashMap;

use inkwell::attributes::AttributeLoc;
use inkwell::types::FunctionType;
use inkwell::values::BasicValue;
use inkwell::values::BasicValueEnum;
use inkwell::AddressSpace;

use crate::specialize::tree::{self, Function, Term, Variable};

use super::codegen::CodeGen;

impl<'a> CodeGen<'a> {
    fn get_function_type(&mut self, term: &Function) -> FunctionType<'a> {
        let mut params = vec![self.i64.into(); term.params.len()];
        params.insert(0, self.ptr.into());

        self.i64.fn_type(&params, false)
    }

    fn lambda_guard<T>(&mut self, fun: impl FnOnce(&mut Self) -> T) -> T {
        let mut names = FxHashMap::default();
        std::mem::swap(&mut self.function_ctx.names, &mut names);
        let result = fun(self);
        std::mem::swap(&mut self.function_ctx.names, &mut names);
        result
    }

    fn generate_name(&mut self, arity: u8) -> String {
        let name_stack = self.function_ctx.name_stack.join(".");

        let local_name = match self.function_ctx.anonymous {
            Some(ref name) => format!("<{}.{name} as soft.function(arity: {arity})>", name_stack),
            None => format!("<{}.local as soft.function(arity: {arity})>", name_stack),
        };

        local_name
    }

    fn build_closure(
        &mut self,
        term: &Function<'a>,
        function: inkwell::values::FunctionValue<'a>,
    ) -> BasicValueEnum<'a> {
        let arity = term.params.len() as u8;
        let lambda = function.as_global_value().as_pointer_value();

        let lambda = self
            .llvm_ctx
            .builder
            .build_pointer_cast(lambda, self.ptr, "");
        let arity = self.i8.const_int(arity as u64, false);

        self.prim__function(lambda.as_basic_value_enum(), arity.into())
    }

    pub fn compile_function(&mut self, term: &Function<'a>) -> BasicValueEnum<'a> {
        self.lambda_guard(|this| {
            let previous_block = this.function_ctx.basic_block();

            let ty = this.get_function_type(term);
            let name = this.generate_name(term.params.len() as u8);

            let function = this.llvm_ctx.module.add_function(&name, ty, None);
            function.get_nth_param(0).unwrap().set_name("$env");

            for (i, param) in function.get_params().iter().skip(1).enumerate() {
                let symbol = &term.params[i];
                param.set_name(symbol.name());
                this.function_ctx.names.insert(symbol.name().into(), *param);
            }

            function.add_attribute(AttributeLoc::Function, this.attr("nobuiltin"));
            function.add_attribute(AttributeLoc::Function, this.attr("uwtable"));
            function.add_attribute(AttributeLoc::Param(0), this.attr_value("align", 8));

            let entry_block = this.llvm_ctx.context.append_basic_block(function, "entry");
            this.llvm_ctx.builder.position_at_end(entry_block);

            let value = this.compile_term(&term.body);
            this.llvm_ctx.builder.build_return(Some(&value));

            this.llvm_ctx.builder.position_at_end(previous_block);

            this.build_closure(term, function)
        })
    }
}

impl<'a> CodeGen<'a> {
    fn incorrect_arity_fail(&mut self, expected: u8) -> BasicValueEnum<'a> {
        let message = format!("[err] got {expected} arguments.");

        let message = self
            .llvm_ctx
            .builder
            .build_global_string_ptr(&message, "call.panic");

        self.soft_panic(message.as_pointer_value().as_basic_value_enum())
    }

    pub fn compile_function_call(&mut self, app: &tree::Call<'a>) -> BasicValueEnum<'a> {
        let parent = self.function_ctx.basic_block().get_parent().unwrap();

        let callee = self.compile_term(&app.func);

        let expected_arity = self.i8.const_int(app.args.len() as u64, false);
        let actual_arity = self.prim__get_function_arity(callee);

        let then = self.llvm_ctx.context.append_basic_block(parent, "then");
        let fail = self.llvm_ctx.context.append_basic_block(parent, "fail");

        let arity_check = self.llvm_ctx.builder.build_int_compare(
            inkwell::IntPredicate::EQ,
            expected_arity,
            actual_arity.into_int_value(),
            "arity_check",
        );

        self.llvm_ctx
            .builder
            .build_conditional_branch(arity_check, then, fail);

        self.llvm_ctx.builder.position_at_end(fail);
        self.incorrect_arity_fail(app.args.len() as u8);
        self.llvm_ctx.builder.build_unreachable();

        self.llvm_ctx.builder.position_at_end(then);

        let ptr = self.prim__get_function_ptr(callee);
        let env = self.prim__get_function_env(callee);

        // Set the first parameter as the closure's context
        let mut params = vec![self.i64.into(); app.args.len()];
        params.insert(0, self.ptr.into());

        let ty = self.i64.fn_type(&params, false);

        self.function_ctx.set_basic_block(then);

        let ptr = self.llvm_ctx.builder.build_pointer_cast(
            ptr.into_pointer_value(),
            ty.ptr_type(AddressSpace::default()),
            "",
        );

        let mut arguments: Vec<_> = app
            .args
            .iter()
            .map(|term| self.compile_term(term).into())
            .collect_vec();

        arguments.insert(0, env.into());

        self.llvm_ctx
            .builder
            .build_indirect_call(ty, ptr, &arguments, "")
            .try_as_basic_value()
            .left()
            .expect("The result is not a value")
    }
}

impl<'a> CodeGen<'a> {
    pub fn compile_number(&mut self, number: &tree::Number) -> BasicValueEnum<'a> {
        let value = self.i64.const_int(number.value, false).into();
        self.prim__new_u61(value)
    }

    pub fn compile_bool(&mut self, boolean: &tree::Bool) -> BasicValueEnum<'a> {
        if boolean.value {
            self.prim__true()
        } else {
            self.prim__false()
        }
    }

    pub fn compile_variable(&mut self, variable: &Variable) -> BasicValueEnum<'a> {
        match variable {
            Variable::Local { name, .. } => *self.function_ctx.names.get(name.name()).unwrap(),
            Variable::Env { .. } => todo!(),
            Variable::Global { .. } => todo!(),
        }
    }

    pub fn compile_term(&mut self, term: &Term<'a>) -> BasicValueEnum<'a> {
        match term {
            Term::Number(number) => self.compile_number(number),
            Term::CreateClosure(closure) => self.compile_function(closure),
            Term::Variable(variable) => self.compile_variable(variable),
            Term::Call(call) => self.compile_function_call(call),
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
            Term::Binary(_) => todo!(),
            Term::Str(_) => todo!(),
            Term::Bool(bool) => self.compile_bool(bool),
            Term::Let(_) => todo!(),
            Term::Lambda(_) => todo!(),
            Term::Block(_) => todo!(),
            Term::Quote(_) => todo!(),
            Term::If(_) => todo!(),
        }
    }
}
