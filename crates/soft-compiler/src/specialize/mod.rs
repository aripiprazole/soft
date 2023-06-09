//! This module specifies an s-expression into a more compiler-friendly tree. The step transforms an
//! s-expression into a specialized tree and then runs a closure conversion algorithm on it.

use std::ops::Range;

use itertools::Itertools;

use crate::{expr::*, location::*, specialize::tree::*};

pub mod closure;
pub mod free;
pub mod substitute;
pub mod tree;

/// The specialization context. It's used to keep track of local definitions in order to optimize
/// it further on the future. Unbound variables are treated like variables that need to be searched
/// on the context.
#[derive(Clone, Default)]
pub struct Ctx<'a> {
    params: im_rc::HashMap<Symbol<'a>, usize>,
    count: usize,
}

/// Type synonym to a range of locations
type Span = Range<Loc>;

/// Expressions is a mutable slice of expressions
type Exprs<'a, 'b> = Vec<Expr<'a>>;

impl<'a> Ctx<'a> {
    /// Extends a context with a set of symbols
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

    /// Add a symbol to the context
    fn add(&self, name: Symbol<'a>) -> Ctx<'a> {
        let mut new_ctx = self.clone();
        new_ctx.params.insert(name, self.count);
        new_ctx.count += 1;
        new_ctx
    }

    /// Checks if the size of a list is equal to a given size
    pub fn check_size(&self, args: &Exprs<'a, '_>, size: usize) -> Option<()> {
        if args.len() != size {
            return None;
        }

        Some(())
    }

    /// Specializes a sequence of expression into a vector of terms
    pub fn specialize_iter<'b, I>(&self, iter: I) -> Vec<Term<'a>>
    where
        I: IntoIterator<Item = Expr<'a>>,
        'a: 'b,
    {
        iter.into_iter().map(|x| self.specialize(x)).collect_vec()
    }

    /// Specializes the operation using the name of the function
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

    /// Specializes an operation expression into a term
    pub fn specialize_operation_expr(
        &self,
        op: OperationKind,
        span: Span,
        args: Exprs<'a, '_>,
    ) -> Option<Term<'a>> {
        let new_args = self.specialize_iter(args);
        Some(Term::new(span, TermKind::Operation(op, new_args)))
    }

    /// Specializes an if expression into a term
    pub fn specialize_if(&self, span: Span, args: Exprs<'a, '_>) -> Option<Term<'a>> {
        self.check_size(&args, 3)?;

        let mut iter = args.into_iter();

        Some(Term::new(
            span,
            TermKind::If(
                Box::new(self.specialize(iter.next().unwrap())),
                Box::new(self.specialize(iter.next().unwrap())),
                Box::new(self.specialize(iter.next().unwrap())),
            ),
        ))
    }

    /// Specializes an expression into a quoted term
    pub fn specialize_quote(&self, span: Span, args: Exprs<'a, '_>) -> Option<Term<'a>> {
        self.check_size(&args, 1)?;
        Some(Term::new(span, TermKind::Quote(Box::new(args[0].clone()))))
    }

    /// Specializes a sequence of operations (a block) into a term
    pub fn specialize_block(&self, span: Span, args: Exprs<'a, '_>) -> Option<Term<'a>> {
        Some(Term::new(span, TermKind::Block(self.specialize_iter(args))))
    }

    /// Specializes a let expression with multiple binders into a term
    pub fn specialize_let(&self, span: Span, mut args: Exprs<'a, '_>) -> Option<Term<'a>> {
        if args.is_empty() {
            return None;
        }

        let mut ctx = self.clone();
        let mut new_args = Vec::with_capacity(args.len() - 1);

        let last = args.pop();

        for arg in args {
            let list = arg.get_list()?;
            self.check_size(&list, 2)?;

            let mut iter = list.into_iter();

            let name = Symbol::new(iter.next().unwrap().get_identifier()?);
            let value = ctx.specialize(iter.next().unwrap());

            new_args.push((name.clone(), value));

            ctx = self.add(name.clone());
        }

        let next = Box::new(ctx.specialize(last.unwrap()));

        Some(Term::new(span, TermKind::Let(new_args, next)))
    }

    /// Specializes a lambda expression into a term
    pub fn specialize_lambda(&self, span: Span, args: Exprs<'a, '_>) -> Option<Term<'a>> {
        if args.len() != 2 {
            return None;
        }

        let mut iter = args.into_iter();

        let params = iter.next().unwrap().get_list()?;

        let mut parameters = params
            .iter()
            .map(|x| x.get_identifier())
            .map(|x| x.map(Symbol::new))
            .collect::<Option<Vec<_>>>()?;

        let mut is_variadic = false;

        if let Some(res) = parameters.last_mut() {
            if res.name().starts_with('&') {
                *res = Symbol::new(&res.name()[1..]);
                is_variadic = true;
            }
        }

        let ctx = self.extend(parameters.iter().cloned());

        let def = Definition {
            is_variadic,
            parameters,
            body: Box::new(ctx.specialize(iter.next().unwrap())),
        };

        Some(Term::new(span, TermKind::Lambda(def, IsLifted::No)))
    }

    /// Specializes a set expression (that can be a macro and) into a term
    pub fn specialize_set(
        &self,
        span: Span,
        args: Exprs<'a, '_>,
        is_macro: IsMacro,
    ) -> Option<Term<'a>> {
        self.check_size(&args, 2);

        let cloned = args[1].clone();

        let mut iter = args.into_iter();
        let name = Symbol::new(iter.next().unwrap().get_identifier()?);
        let value = self.specialize(iter.next().unwrap());

        Some(Term::new(
            span,
            TermKind::Set(name, Box::new(cloned), Box::new(value), is_macro),
        ))
    }

    /// Specializes a call expression into a term other side it fallbacks to a cons-cell abstraction
    fn specialize_call_expr(self, loc: Span, list: Exprs<'a, '_>) -> Term<'a> {
        match self.specialize_list(loc.clone(), &list) {
            Some(x) => x,
            None => self.fallback_call(loc, list),
        }
    }

    /// Specializes a local or global variable into a term
    fn specialize_var(&self, loc: Span, name: &'a str) -> Term<'a> {
        let symbol = Symbol::new(name);

        let var = if let Some(place) = self.params.get(&symbol) {
            VariableKind::Local(*place, symbol)
        } else {
            VariableKind::Global(symbol)
        };

        Spanned::new(loc, TermKind::Variable(var))
    }

    /// Specializes a list with at least one argument into something more specific than a cons-cell
    /// abstraction.
    pub fn specialize_list(&self, span: Span, args: &Exprs<'a, '_>) -> Option<Term<'a>> {
        let mut iter = args.iter();
        let name = iter.next().unwrap().get_identifier()?;
        self.specialize_call(span, name, iter.as_slice())
    }

    pub fn specialize_call(&self, span: Span, name: &str, args: &[Expr<'a>]) -> Option<Term<'a>> {
        match name {
            "let" => self.specialize_let(span, args.to_vec()),
            "lambda" => self.specialize_lambda(span, args.to_vec()),
            "set!" => self.specialize_set(span, args.to_vec(), IsMacro::No),
            "setm!" => self.specialize_set(span, args.to_vec(), IsMacro::Yes),
            "block" => self.specialize_block(span, args.to_vec()),
            "quote" => self.specialize_quote(span, args.to_vec()),
            "if" => self.specialize_if(span, args.to_vec()),
            _ => {
                if let Some(name) = self.specialize_operation(name) {
                    self.specialize_operation_expr(name, span, args.to_vec())
                } else {
                    None
                }
            }
        }
    }

    /// Creates a simple cons-cell call if it's not possible to specialize the call into something
    pub fn fallback_call(self, loc: Span, exprs: Exprs<'a, '_>) -> Term<'a> {
        let mut iter = exprs.into_iter();
        let fun = self.clone().specialize(iter.next().unwrap());
        let args = iter.map(|x| self.clone().specialize(x));

        Spanned::new(loc, TermKind::Call(Box::new(fun), args.collect()))
    }

    /// Specializes an s-expression [Expr] into a [Term] that contains a more metadata.
    pub fn specialize(&self, expr: Expr<'a>) -> Term<'a> {
        use ExprKind::*;

        let loc = expr.range.clone();

        match expr.data {
            Identifier(name) => self.specialize_var(loc, name),
            List(list) if !list.is_empty() => self.clone().specialize_call_expr(loc, list),
            List(_) => Term::new(loc, TermKind::Prim(PrimKind::Nil)),
            Number(num) => Term::new(loc, TermKind::Number(num)),
            Atom(name) => Term::new(loc, TermKind::Atom(name)),
            String(str) => Term::new(loc, TermKind::String(str)),
        }
    }
}

impl<'a> Expr<'a> {
    /// Entry point for specialization. It gets a raw expression and turns it into a [Term] that contains
    /// more metadata is classified.
    pub fn specialize(self) -> Term<'a> {
        let state = Ctx::default();
        state.specialize(self)
    }
}
