//! This module specifies an s-expression into a more compiler-friendly tree. The step transforms an
//! s-expression into a specialized tree and then runs a closure convertion algorithm on it.

use std::ops::Range;

use itertools::Itertools;

use crate::{
    location::{Loc, Spanned},
    parser::syntax::{Expr, ExprKind},
    spec::tree::{PrimKind, TermKind, VariableKind},
};

use self::tree::{Definition, IsLifted, IsMacro, OperationKind, Symbol, Term};

pub mod tree;

/// The specialization context. It's used to keep track of local definitions in order to optimize
/// it further on the future. Unbound variables are treated like variables that need to be searched
/// on the context.
#[derive(Clone, Default)]
pub struct Ctx<'a> {
    params: im_rc::HashMap<Symbol<'a>, usize>,
    count: usize,
}

type Span = Range<Loc>;

type Exprs<'a, 'b> = &'b mut [Expr<'a>];

impl<'a> Ctx<'a> {
    fn extend<I: IntoIterator<Item = Symbol<'a>>>(&self, iter: I) -> Ctx<'a> {
        let mut clone = self.clone();

        let params = iter
            .into_iter()
            .enumerate()
            .map(|x| (x.1, x.0 + clone.count))
            .collect::<Vec<_>>();

        let size = params.len();

        clone.params.extend(params.into_iter());
        clone.count += size;
        clone
    }

    fn add(&self, name: Symbol<'a>) -> Ctx<'a> {
        let mut new_ctx = self.clone();
        new_ctx.params.insert(name, self.count);
        new_ctx.count += 1;
        new_ctx
    }

    pub fn check_size(&self, args: Exprs<'a, '_>, size: usize) -> Option<()> {
        if args.len() != size {
            return None;
        }

        Some(())
    }

    pub fn specialize_iter<'b, I>(&self, iter: I) -> Vec<Term<'a>>
    where
        I: IntoIterator<Item = &'b mut Expr<'a>>,
        'a: 'b,
    {
        iter.into_iter().map(|x| self.specialize(x)).collect_vec()
    }

    pub fn specialize_operation(&self, name: &str) -> Option<OperationKind> {
        use OperationKind::*;

        match name {
            "+" => Some(Add),
            "-" => Some(Sub),
            "*" => Some(Mul),
            "/" => Some(Div),
            "%" => Some(Mod),
            "<<" => Some(Shl),
            ">>" => Some(Shr),
            "&" => Some(And),
            "^" => Some(Xor),
            "|" => Some(Or),
            "!" => Some(Not),
            "==" => Some(Eql),
            "!=" => Some(Neq),
            ">" => Some(Gtn),
            ">=" => Some(Gte),
            "<" => Some(Ltn),
            "=<" => Some(Lte),
            "&&" => Some(LAnd),
            "||" => Some(LOr),
            _ => None,
        }
    }

    pub fn specialize_op(&self, span: Span, name: &str, args: &mut [Expr<'a>]) -> Option<Term<'a>> {
        let op = self.specialize_operation(name)?;
        let new_args = self.specialize_iter(args);
        Some(Term::new(span, TermKind::Operation(op, new_args)))
    }

    pub fn specialize_if(&self, span: Span, args: Exprs<'a, '_>) -> Option<Term<'a>> {
        if args.len() != 3 {
            return None;
        }

        Some(Term::new(
            span,
            TermKind::If(
                Box::new(self.specialize(&mut args[0])),
                Box::new(self.specialize(&mut args[1])),
                Box::new(self.specialize(&mut args[2])),
            ),
        ))
    }

    pub fn specialize_quote(&self, span: Span, args: Exprs<'a, '_>) -> Option<Term<'a>> {
        if args.len() != 1 {
            return None;
        }

        Some(Term::new(span, TermKind::Quote(Box::new(args[0].take()))))
    }

    pub fn specialize_block(&self, span: Span, args: Exprs<'a, '_>) -> Option<Term<'a>> {
        Some(Term::new(span, TermKind::Block(self.specialize_iter(args))))
    }

    pub fn specialize_let(&self, span: Span, args: Exprs<'a, '_>) -> Option<Term<'a>> {
        if args.is_empty() {
            return None;
        }

        let mut ctx = self.clone();
        let mut new_args = Vec::new();

        let size = args.len();

        for arg in &mut args[0..size - 1] {
            let list = arg.get_list()?;
            self.check_size(list, 2)?;

            let name = Symbol::new(list[0].get_identifier()?);
            let value = ctx.specialize(&mut list[1]);

            new_args.push((name.clone(), value));

            ctx = self.add(name.clone());
        }

        let next = Box::new(ctx.specialize(&mut args[size - 1]));

        Some(Term::new(span, TermKind::Let(new_args, next)))
    }

    pub fn specialize_lambda(&self, span: Span, args: Exprs<'a, '_>) -> Option<Term<'a>> {
        if args.len() < 2 {
            return None;
        }

        let params = args[0].get_list()?;

        let parameters: Option<Vec<Symbol<'a>>> = params
            .iter()
            .map(|x| x.get_identifier())
            .map(|x| x.map(Symbol::new))
            .collect();

        let mut parameters = parameters?;

        let mut is_variadic = false;

        if let Some(res) = parameters.last_mut() {
            if res.debug_name.starts_with('&') {
                *res = Symbol::new(&res.debug_name[1..]);
                is_variadic = true;
            }
        }

        let ctx = self.extend(parameters.iter().cloned());

        let def = Definition {
            is_variadic,
            parameters,
            body: ctx.specialize_iter(&mut args[1..]),
        };

        Some(Term::new(span, TermKind::Lambda(def, IsLifted::No)))
    }

    pub fn specialize_set(
        &self,
        span: Span,
        args: Exprs<'a, '_>,
        is_macro: IsMacro,
    ) -> Option<Term<'a>> {
        if args.len() != 2 {
            return None;
        }

        let name = Symbol::new(args[0].get_identifier()?);
        let value = self.specialize(&mut args[1]);

        Some(Term::new(
            span,
            TermKind::Set(name, Box::new(value), is_macro),
        ))
    }

    pub fn specialize_call(&self, span: Span, name: &str, args: Exprs<'a, '_>) -> Option<Term<'a>> {
        match name {
            "let" => self.specialize_let(span, args),
            "lambda" => self.specialize_lambda(span, args),
            "set!" => self.specialize_set(span, args, IsMacro::No),
            "setm!" => self.specialize_set(span, args, IsMacro::Yes),
            "block" => self.specialize_block(span, args),
            "quote" => self.specialize_quote(span, args),
            "if" => self.specialize_if(span, args),
            _ => self.specialize_op(span, name, args),
        }
    }

    /// Specializes a list with at least one argument into something more specific than a cons-cell
    /// abstraction.
    pub fn specialize_list(&self, span: Span, args: Exprs<'a, '_>) -> Option<Term<'a>> {
        let name = args[0].get_identifier()?;
        self.specialize_call(span, name, &mut args[1..])
    }

    pub fn fallback_call(self, loc: Span, exprs: Exprs<'a, '_>) -> Term<'a> {
        let mut iter = exprs.iter_mut();
        let fun = self.clone().specialize(iter.next().unwrap());
        let args = iter.map(|x| self.clone().specialize(x));

        Spanned::new(loc, TermKind::Call(Box::new(fun), args.collect()))
    }

    fn specialize_call_expr(self, loc: Span, list: Exprs<'a, '_>) -> Term<'a> {
        self.specialize_list(loc.clone(), list)
            .unwrap_or_else(|| self.fallback_call(loc, list))
    }

    fn specialize_var(&self, loc: Span, name: &'a str) -> Term<'a> {
        let symbol = Symbol::new(name);

        let var = if let Some(place) = self.params.get(&symbol) {
            VariableKind::Local(*place, symbol)
        } else {
            VariableKind::Global(symbol)
        };

        Spanned::new(loc, TermKind::Variable(var))
    }

    /// Specializes an s-expression [Expr] into a [Term] that contains a more metadata.
    pub fn specialize(&self, expr: &mut Expr<'a>) -> Term<'a> {
        use ExprKind::*;

        let loc = expr.loc.clone();

        match &mut expr.data {
            Identifier(name) => self.specialize_var(loc, name),
            List(list) if !list.is_empty() => self.clone().specialize_call_expr(loc, list),
            List(_) => Term::new(loc, TermKind::Prim(PrimKind::Nil)),
            Number(num) => Term::new(loc, TermKind::Number(*num)),
            Atom(name) => Term::new(loc, TermKind::Atom(name)),
            String(str) => Term::new(loc, TermKind::String(str)),
        }
    }
}

/// Entry point for specialization. It gets a raw expression and turns it into a [Term] that contains
/// more metadata is classified.
pub fn specialize(mut expr: Expr) -> Term {
    let state = Ctx::default();
    state.specialize(&mut expr)
}
