//! This module specifies an s-expression into a more compiler-friendly tree. The step transforms an
//! s-expression into a specialized tree and then runs a closure conversion algorithm on it.

use crate::expr::{Expr, ExprKind};

use self::tree::*;

pub mod closure;
pub mod tree;

#[derive(Default, Clone)]
pub struct Ctx {
    names: im_rc::HashMap<String, usize>,
    counter: usize,
}

impl Ctx {
    pub fn add(&mut self, name: String) -> usize {
        let count = self.counter;
        self.counter += 1;
        self.names.insert(name, count);
        count
    }

    pub fn lookup(&self, name: &str) -> Option<usize> {
        self.names.get(name).copied()
    }
}

impl<'a> Expr<'a> {
    pub fn assert_size(&self, size: usize) -> Option<&[Self]> {
        if let ExprKind::List(list) = &self.data {
            if list.len() == size {
                return Some(list);
            }
        }
        None
    }

    pub fn at_least_size(&self, size: usize) -> Option<&[Self]> {
        if let ExprKind::List(list) = &self.data {
            if list.len() >= size {
                return Some(list);
            }
        }
        None
    }

    pub fn keyword(&self, keyword: &str) -> Option<()> {
        if let ExprKind::Identifier(str) = self.data {
            if str == keyword {
                return Some(());
            }
        }
        None
    }

    pub fn specialize<T: Specialize<'a>>(&self, ctx: Ctx) -> Option<T> {
        T::specialize(self, ctx)
    }

    pub fn unspecialized_one_layer(&self, ctx: Ctx) -> Term<'a> {
        match &self.data {
            ExprKind::List(ls) => Self::fallback_call(ls, ctx),
            ExprKind::Atom(atom) => Term::Atom(Atom {
                name: Symbol::new(atom.to_string()),
            }),
            ExprKind::Identifier(id) => Term::Variable(Variable::Global {
                name: Symbol::new(id.to_string()),
            }),
            ExprKind::Number(n) => Term::Number(Number { value: *n }),
            ExprKind::String(s) => Term::Str(Str { value: s }),
        }
    }

    pub fn specialize_fallback(&self, ctx: Ctx) -> Term<'a> {
        self.specialize(ctx.clone())
            .unwrap_or_else(|| Self::unspecialized_one_layer(self, ctx))
    }

    pub fn fallback_call(list: &[Expr<'a>], ctx: Ctx) -> Term<'a> {
        let head = list[0].specialize_fallback(ctx.clone());
        let mut tail = Vec::new();

        for expr in list[1..].iter() {
            tail.push(expr.specialize_fallback(ctx.clone()));
        }

        Term::Call(Call {
            func: Box::new(head),
            args: tail,
        })
    }

    pub fn to_term(&self) -> Term<'a> {
        Expr::specialize_fallback(self, Ctx::default())
    }
}

pub trait Specialize<'a>: Into<Term<'a>> {
    fn specialize(expr: &Expr<'a>, ctx: Ctx) -> Option<Self>;
}

impl<'a> Specialize<'a> for TypeOf<'a> {
    fn specialize(expr: &Expr<'a>, ctx: Ctx) -> Option<Self> {
        let list = expr.assert_size(2)?;
        list[0].keyword("type-of")?;
        Some(Self {
            expr: Box::new(list[1].specialize(ctx)?),
        })
    }
}

impl<'a> Specialize<'a> for Vector<'a> {
    fn specialize(expr: &Expr<'a>, ctx: Ctx) -> Option<Self> {
        let list = expr.at_least_size(1)?;
        list[0].keyword("vector")?;
        Some(Self {
            elements: list[1..]
                .iter()
                .map(|expr| expr.specialize(ctx.clone()))
                .collect::<Option<_>>()?,
        })
    }
}

impl<'a> Specialize<'a> for Cons<'a> {
    fn specialize(expr: &Expr<'a>, ctx: Ctx) -> Option<Self> {
        let list = expr.assert_size(3)?;
        list[0].keyword("cons")?;
        Some(Self {
            head: Box::new(list[1].specialize(ctx.clone())?),
            tail: Box::new(list[2].specialize(ctx)?),
        })
    }
}

impl<'a> Specialize<'a> for Nil {
    fn specialize(expr: &Expr<'a>, _: Ctx) -> Option<Self> {
        let list = expr.assert_size(1)?;
        list[0].keyword("nil")?;
        Some(Self)
    }
}

impl<'a> Specialize<'a> for Head<'a> {
    fn specialize(expr: &Expr<'a>, ctx: Ctx) -> Option<Self> {
        let list = expr.assert_size(2)?;
        list[0].keyword("head")?;
        Some(Self {
            list: Box::new(list[1].specialize(ctx)?),
        })
    }
}

impl<'a> Specialize<'a> for Tail<'a> {
    fn specialize(expr: &Expr<'a>, ctx: Ctx) -> Option<Self> {
        let list = expr.assert_size(2)?;
        list[0].keyword("tail")?;
        Some(Self {
            list: Box::new(list[1].specialize(ctx)?),
        })
    }
}

impl<'a> Specialize<'a> for IsNil<'a> {
    fn specialize(expr: &Expr<'a>, ctx: Ctx) -> Option<Self> {
        let list = expr.assert_size(2)?;
        list[0].keyword("is-nil")?;
        Some(Self {
            list: Box::new(list[1].specialize(ctx)?),
        })
    }
}

impl<'a> Specialize<'a> for VectorIndex<'a> {
    fn specialize(expr: &Expr<'a>, ctx: Ctx) -> Option<Self> {
        let list = expr.assert_size(3)?;
        list[0].keyword("vector-index")?;
        Some(Self {
            vector: Box::new(list[1].specialize(ctx.clone())?),
            index: Box::new(list[2].specialize(ctx)?),
        })
    }
}

impl<'a> Specialize<'a> for VectorLen<'a> {
    fn specialize(expr: &Expr<'a>, ctx: Ctx) -> Option<Self> {
        let list = expr.assert_size(2)?;
        list[0].keyword("vector-len")?;
        Some(Self {
            vector: Box::new(list[1].specialize(ctx)?),
        })
    }
}

impl<'a> Specialize<'a> for VectorPush<'a> {
    fn specialize(expr: &Expr<'a>, ctx: Ctx) -> Option<Self> {
        let list = expr.assert_size(3)?;
        list[0].keyword("vector-push")?;
        Some(Self {
            vector: Box::new(list[1].specialize(ctx.clone())?),
            element: Box::new(list[2].specialize(ctx)?),
        })
    }
}

impl<'a> Specialize<'a> for BoxTerm<'a> {
    fn specialize(expr: &Expr<'a>, ctx: Ctx) -> Option<Self> {
        let list = expr.assert_size(2)?;
        list[0].keyword("box")?;
        Some(Self {
            term: Box::new(list[1].specialize(ctx)?),
        })
    }
}

impl<'a> Specialize<'a> for UnboxTerm<'a> {
    fn specialize(expr: &Expr<'a>, ctx: Ctx) -> Option<Self> {
        let list = expr.assert_size(2)?;
        list[0].keyword("unbox")?;
        Some(Self {
            term: Box::new(list[1].specialize(ctx)?),
        })
    }
}
impl<'a> Specialize<'a> for Binary<'a> {
    fn specialize(expr: &Expr<'a>, ctx: Ctx) -> Option<Self> {
        let list = expr.assert_size(4)?;

        list[0].keyword("binary")?;

        let op = match list[1].get_identifier()? {
            "+" => OperationKind::Add,
            "-" => OperationKind::Sub,
            "*" => OperationKind::Mul,
            "/" => OperationKind::Div,
            "%" => OperationKind::Mod,
            "<<" => OperationKind::Shl,
            ">>" => OperationKind::Shr,
            "&" => OperationKind::And,
            "^" => OperationKind::Xor,
            "|" => OperationKind::Or,
            "!" => OperationKind::Not,
            "==" => OperationKind::Eql,
            "!=" => OperationKind::Neq,
            ">" => OperationKind::Gtn,
            ">=" => OperationKind::Gte,
            "<" => OperationKind::Ltn,
            "=<" => OperationKind::Lte,
            "&&" => OperationKind::LAnd,
            "||" => OperationKind::LOr,
            _ => return None,
        };

        let left = Box::new(list[2].specialize(ctx.clone())?);
        let right = Box::new(list[3].specialize(ctx)?);

        Some(Self { op, left, right })
    }
}

impl<'a> Specialize<'a> for Number {
    fn specialize(expr: &Expr<'a>, _: Ctx) -> Option<Self> {
        match expr.data {
            ExprKind::Number(value) => Some(Self { value }),
            _ => None,
        }
    }
}

impl<'a> Specialize<'a> for Str<'a> {
    fn specialize(expr: &Expr<'a>, _: Ctx) -> Option<Self> {
        match expr.data {
            ExprKind::String(value) => Some(Self { value }),
            _ => None,
        }
    }
}

impl<'a> Specialize<'a> for Bool {
    fn specialize(expr: &Expr<'a>, _: Ctx) -> Option<Self> {
        match expr.data {
            ExprKind::Identifier("true") => Some(Self { value: true }),
            ExprKind::Identifier("false") => Some(Self { value: false }),
            _ => None,
        }
    }
}

impl<'a> Specialize<'a> for Variable {
    fn specialize(expr: &Expr<'a>, ctx: Ctx) -> Option<Self> {
        match expr.data {
            ExprKind::Identifier(name) => {
                if let Some(index) = ctx.lookup(name) {
                    Some(Variable::Local {
                        index,
                        name: Symbol::new(name.to_string()),
                    })
                } else {
                    Some(Variable::Global {
                        name: Symbol::new(name.to_string()),
                    })
                }
            }
            _ => None,
        }
    }
}

impl<'a> Specialize<'a> for Let<'a> {
    fn specialize(expr: &Expr<'a>, mut ctx: Ctx) -> Option<Self> {
        let mut list = expr.at_least_size(3)?.to_vec();
        list[0].keyword("let")?;
        let mut bindings = Vec::new();
        let last = list.pop().unwrap();
        for binding in list[1..].iter() {
            let list = binding.assert_size(2)?;
            let name = Symbol::new(list[0].get_identifier()?.to_ascii_lowercase());
            let term = list[1].specialize(ctx.clone())?;
            ctx.add(name.name().to_string());
            bindings.push((name, term));
        }
        let body = Box::new(last.specialize(ctx)?);
        Some(Self { bindings, body })
    }
}

impl<'a> Specialize<'a> for Lambda<'a> {
    fn specialize(expr: &Expr<'a>, mut ctx: Ctx) -> Option<Self> {
        let list = expr.assert_size(3)?;
        list[0].keyword("lambda")?;

        let args = list[1].at_least_size(0)?;
        let mut new_args = Vec::new();

        ctx.counter = 0;

        for arg in args.iter() {
            let name = Symbol::new(arg.get_identifier()?.to_string());
            ctx.add(name.name().to_string());
            new_args.push(name);
        }

        let body = Box::new(list.last()?.specialize(ctx)?);
        Some(Self {
            args: new_args,
            body,
        })
    }
}

impl<'a> Specialize<'a> for Block<'a> {
    fn specialize(expr: &Expr<'a>, ctx: Ctx) -> Option<Self> {
        let list = expr.at_least_size(2)?;
        list[0].keyword("block")?;

        let mut body = Vec::new();

        for expr in list[1..].iter() {
            body.push(expr.specialize(ctx.clone())?);
        }

        Some(Self { body })
    }
}

impl<'a> Specialize<'a> for Quote<'a> {
    fn specialize(expr: &Expr<'a>, _: Ctx) -> Option<Self> {
        let list = expr.assert_size(2)?;
        list[0].keyword("quote")?;
        let value = Box::new(list[1].clone());
        Some(Self { value })
    }
}

impl<'a> Specialize<'a> for If<'a> {
    fn specialize(expr: &Expr<'a>, ctx: Ctx) -> Option<Self> {
        let list = expr.assert_size(4)?;
        list[0].keyword("if")?;
        let cond = Box::new(list[1].specialize(ctx.clone())?);
        let then = Box::new(list[2].specialize(ctx.clone())?);
        let else_ = Box::new(list[3].specialize(ctx)?);
        Some(Self { cond, then, else_ })
    }
}

impl<'a> Specialize<'a> for Term<'a> {
    fn specialize(expr: &Expr<'a>, ctx: Ctx) -> Option<Self> {
        match &expr.data {
            ExprKind::List(list) => {
                if list.is_empty() {
                    Some(Self::Nil(Nil))
                } else if let Some(name) = list[0].get_identifier() {
                    match name {
                        "vector" => Vector::specialize(expr, ctx).map(Self::Vector),
                        "cons" => Cons::specialize(expr, ctx).map(Self::Cons),
                        "nil" => Some(Self::Nil(Nil)),
                        "head" => Head::specialize(expr, ctx).map(Self::Head),
                        "tail" => Tail::specialize(expr, ctx).map(Self::Tail),
                        "is-nil" => IsNil::specialize(expr, ctx).map(Self::IsNil),
                        "vector-index" => VectorIndex::specialize(expr, ctx).map(Self::VectorIndex),
                        "vector-len" => VectorLen::specialize(expr, ctx).map(Self::VectorLen),
                        "vector-push" => VectorPush::specialize(expr, ctx).map(Self::VectorPush),
                        "box" => BoxTerm::specialize(expr, ctx).map(Self::Box),
                        "unbox" => UnboxTerm::specialize(expr, ctx).map(Self::Unbox),
                        "let" => Let::specialize(expr, ctx).map(Self::Let),
                        "lambda" => Lambda::specialize(expr, ctx).map(Self::Lambda),
                        "block" => Block::specialize(expr, ctx).map(Self::Block),
                        "binary" => Binary::specialize(expr, ctx).map(Self::Binary),
                        "quote" => Quote::specialize(expr, ctx).map(Self::Quote),
                        "if" => If::specialize(expr, ctx).map(Self::If),
                        _ => Some(Expr::fallback_call(list, ctx)),
                    }
                } else {
                    Some(expr.unspecialized_one_layer(ctx))
                }
            }
            ExprKind::Atom(n) => Some(Self::Atom(Atom {
                name: Symbol::new(n.to_string()),
            })),
            ExprKind::Number(n) => Some(Self::Number(Number { value: *n })),
            ExprKind::String(s) => Some(Self::Str(Str { value: s })),
            ExprKind::Identifier(name) => match *name {
                "true" => Some(Self::Bool(Bool { value: true })),
                "false" => Some(Self::Bool(Bool { value: false })),
                _ => expr.specialize(ctx).map(Self::Variable),
            },
        }
    }
}
