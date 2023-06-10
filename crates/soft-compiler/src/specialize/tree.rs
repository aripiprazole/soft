//! This module is a agglomeration of a lot of different trees into a single tree. It's used to
//! generate a specialized version of the s-expression tree into one that is optimized for the
//! backend.

use core::fmt;
use std::{fmt::Display, hash::Hash};

use itertools::Itertools;

use crate::expr::{Expr, ExprKind};

/// Symbol is a struct that makes string comparisons O(1) by comparing it's memoized hash, instead
/// of it's names, but still uses the names to better error handling and debug.
#[derive(Clone, Eq, Debug)]
pub struct Symbol {
    debug_name: String,
    hash: usize,
}

impl PartialEq for Symbol {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        self.hash == other.hash
    }
}

impl Hash for Symbol {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        // Reuses the hash symbol's generated hash
        state.write_usize(self.hash);
    }
}

impl Symbol {
    /// Creates a new symbol, and generates a hash for it using [fxhash].
    pub fn new(debug_name: String) -> Self {
        Self {
            hash: fxhash::hash(&debug_name),
            debug_name,
        }
    }

    pub fn name(&self) -> &str {
        &self.debug_name
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum OperationKind {
    Add, // +
    Sub, // -
    Mul, // *
    Div, // /
    Mod, // %
    Shl, // <<
    Shr, // >>
    And, // &
    Xor, // ^
    Or,  // |

    // Logical operations
    Not, // !

    Eql, // ==
    Neq, // !=
    Gtn, // >
    Gte, // >=
    Ltn, // <
    Lte, // =<

    LAnd, // &&
    LOr,  // ||
}

#[derive(Debug)]
pub struct TypeOf<'a> {
    pub expr: Box<Term<'a>>,
}

#[derive(Debug)]
pub struct Atom {
    pub name: Symbol,
}

#[derive(Debug)]
pub struct Vector<'a> {
    pub elements: Vec<Term<'a>>,
}

#[derive(Debug)]
pub struct Cons<'a> {
    pub head: Box<Term<'a>>,
    pub tail: Box<Term<'a>>,
}

#[derive(Debug)]
pub struct Nil;

#[derive(Debug)]
pub struct Head<'a> {
    pub list: Box<Term<'a>>,
}

#[derive(Debug)]
pub struct Tail<'a> {
    pub list: Box<Term<'a>>,
}

#[derive(Debug)]
pub struct IsNil<'a> {
    pub list: Box<Term<'a>>,
}

#[derive(Debug)]
pub struct VectorIndex<'a> {
    pub vector: Box<Term<'a>>,
    pub index: Box<Term<'a>>,
}

#[derive(Debug)]
pub struct VectorLen<'a> {
    pub vector: Box<Term<'a>>,
}

#[derive(Debug)]
pub struct VectorPush<'a> {
    pub vector: Box<Term<'a>>,
    pub element: Box<Term<'a>>,
}

#[derive(Debug)]
pub struct BoxTerm<'a> {
    pub term: Box<Term<'a>>,
}

#[derive(Debug)]
pub struct UnboxTerm<'a> {
    pub term: Box<Term<'a>>,
}

#[derive(Debug)]
pub struct Function<'a> {
    pub env: Vec<Term<'a>>,
    pub params: Vec<Symbol>,
    pub body: Box<Term<'a>>,
}

#[derive(Debug)]
pub struct Binary<'a> {
    pub op: OperationKind,
    pub left: Box<Term<'a>>,
    pub right: Box<Term<'a>>,
}

#[derive(Debug)]
pub struct Number {
    pub value: u64,
}

#[derive(Debug)]
pub struct Str<'a> {
    pub value: &'a str,
}

#[derive(Debug)]
pub struct Bool {
    pub value: bool,
}

#[derive(Debug)]
pub enum Variable {
    Local { name: Symbol, index: usize },
    Env { name: Symbol, index: usize },
    Global { name: Symbol },
}

#[derive(Debug)]
pub struct Let<'a> {
    pub bindings: Vec<(Symbol, Term<'a>)>,
    pub body: Box<Term<'a>>,
}

#[derive(Debug)]
pub struct Lambda<'a> {
    pub args: Vec<Symbol>,
    pub body: Box<Term<'a>>,
}

#[derive(Debug)]
pub struct Block<'a> {
    pub body: Vec<Term<'a>>,
}

#[derive(Debug)]
pub struct Quote<'a> {
    pub value: Box<Expr<'a>>,
}

#[derive(Debug)]
pub struct If<'a> {
    pub cond: Box<Term<'a>>,
    pub then: Box<Term<'a>>,
    pub else_: Box<Term<'a>>,
}

#[derive(Debug)]
pub struct Call<'a> {
    pub func: Box<Term<'a>>,
    pub args: Vec<Term<'a>>,
}

#[derive(Debug)]
pub enum Term<'a> {
    Atom(Atom),
    TypeOf(TypeOf<'a>),
    Vector(Vector<'a>),
    Cons(Cons<'a>),
    Nil(Nil),
    Head(Head<'a>),
    Tail(Tail<'a>),
    IsNil(IsNil<'a>),
    VectorIndex(VectorIndex<'a>),
    VectorLen(VectorLen<'a>),
    VectorPush(VectorPush<'a>),
    Box(BoxTerm<'a>),
    Unbox(UnboxTerm<'a>),
    CreateClosure(Function<'a>),
    Binary(Binary<'a>),
    Number(Number),
    Str(Str<'a>),
    Bool(Bool),
    Variable(Variable),
    Let(Let<'a>),
    Lambda(Lambda<'a>),
    Block(Block<'a>),
    Quote(Quote<'a>),
    If(If<'a>),
    Call(Call<'a>),
}

pub trait Visitor: Sized {
    fn visit_atom(&mut self, atom: &mut Atom) {
        atom.walk(self);
    }

    fn visit_type_of(&mut self, type_of: &mut TypeOf) {
        type_of.walk(self);
    }

    fn visit_vector(&mut self, vector: &mut Vector) {
        vector.walk(self);
    }

    fn visit_cons(&mut self, cons: &mut Cons) {
        cons.walk(self);
    }

    fn visit_nil(&mut self, nil: &mut Nil) {
        nil.walk(self);
    }

    fn visit_head(&mut self, head: &mut Head) {
        head.walk(self);
    }

    fn visit_tail(&mut self, tail: &mut Tail) {
        tail.walk(self);
    }

    fn visit_is_nil(&mut self, is_nil: &mut IsNil) {
        is_nil.walk(self);
    }

    fn visit_vector_index(&mut self, vector_index: &mut VectorIndex) {
        vector_index.walk(self);
    }

    fn visit_vector_len(&mut self, vector_len: &mut VectorLen) {
        vector_len.walk(self);
    }

    fn visit_vector_push(&mut self, vector_push: &mut VectorPush) {
        vector_push.walk(self);
    }

    fn visit_box(&mut self, box_: &mut BoxTerm) {
        box_.walk(self);
    }

    fn visit_unbox(&mut self, unbox: &mut UnboxTerm) {
        unbox.walk(self);
    }
    fn visit_create_closure(&mut self, create_closure: &mut Function) {
        create_closure.walk(self);
    }

    fn visit_binary(&mut self, binary: &mut Binary) {
        binary.walk(self);
    }

    fn visit_number(&mut self, number: &mut Number) {
        number.walk(self);
    }

    fn visit_str(&mut self, str_: &mut Str) {
        str_.walk(self);
    }

    fn visit_bool(&mut self, bool_: &mut Bool) {
        bool_.walk(self);
    }

    fn visit_variable(&mut self, variable: &mut Variable) {
        variable.walk(self);
    }

    fn visit_let(&mut self, let_: &mut Let) {
        let_.walk(self);
    }

    fn visit_lambda(&mut self, lambda: &mut Lambda) {
        lambda.walk(self);
    }

    fn visit_block(&mut self, block: &mut Block) {
        block.walk(self);
    }

    fn visit_quote(&mut self, quote: &mut Quote) {
        quote.walk(self);
    }

    fn visit_if(&mut self, if_: &mut If) {
        if_.walk(self);
    }

    fn visit_call(&mut self, call: &mut Call) {
        call.walk(self);
    }

    fn visit_term(&mut self, expr: &mut Term) {
        expr.walk(self)
    }
}

impl Atom {
    pub fn walk(&mut self, _: &mut impl Visitor) {}
}

impl<'a> TypeOf<'a> {
    pub fn walk(&mut self, _: &mut impl Visitor) {}
}

impl<'a> Vector<'a> {
    pub fn walk(&mut self, visitor: &mut impl Visitor) {
        for term in &mut self.elements {
            visitor.visit_term(term);
        }
    }
}

impl<'a> Cons<'a> {
    pub fn walk(&mut self, visitor: &mut impl Visitor) {
        visitor.visit_term(&mut self.head);
        visitor.visit_term(&mut self.tail);
    }
}

impl Nil {
    pub fn walk(&mut self, _: &mut impl Visitor) {}
}

impl<'a> Head<'a> {
    pub fn walk(&mut self, visitor: &mut impl Visitor) {
        visitor.visit_term(&mut self.list);
    }
}

impl<'a> Tail<'a> {
    pub fn walk(&mut self, visitor: &mut impl Visitor) {
        visitor.visit_term(&mut self.list);
    }
}

impl<'a> IsNil<'a> {
    pub fn walk(&mut self, visitor: &mut impl Visitor) {
        visitor.visit_term(&mut self.list);
    }
}

impl<'a> VectorIndex<'a> {
    pub fn walk(&mut self, visitor: &mut impl Visitor) {
        visitor.visit_term(&mut self.vector);
        visitor.visit_term(&mut self.index);
    }
}

impl<'a> VectorLen<'a> {
    pub fn walk(&mut self, visitor: &mut impl Visitor) {
        visitor.visit_term(&mut self.vector);
    }
}

impl<'a> VectorPush<'a> {
    pub fn walk(&mut self, visitor: &mut impl Visitor) {
        visitor.visit_term(&mut self.vector);
        visitor.visit_term(&mut self.element);
    }
}

impl<'a> BoxTerm<'a> {
    pub fn walk(&mut self, visitor: &mut impl Visitor) {
        visitor.visit_term(&mut self.term);
    }
}

impl<'a> UnboxTerm<'a> {
    pub fn walk(&mut self, visitor: &mut impl Visitor) {
        visitor.visit_term(&mut self.term);
    }
}

impl<'a> Function<'a> {
    pub fn walk(&mut self, visitor: &mut impl Visitor) {
        visitor.visit_term(&mut self.body);
        for arg in &mut self.env {
            visitor.visit_term(arg);
        }
    }
}

impl<'a> Binary<'a> {
    pub fn walk(&mut self, visitor: &mut impl Visitor) {
        visitor.visit_term(&mut self.left);
        visitor.visit_term(&mut self.right);
    }
}

impl Number {
    pub fn walk(&mut self, _: &mut impl Visitor) {}
}

impl<'a> Str<'a> {
    pub fn walk(&mut self, _: &mut impl Visitor) {}
}

impl Bool {
    pub fn walk(&mut self, _: &mut impl Visitor) {}
}

impl Variable {
    pub fn walk(&mut self, _: &mut impl Visitor) {}
}

impl<'a> Let<'a> {
    pub fn walk(&mut self, visitor: &mut impl Visitor) {
        for (_, value) in self.bindings.iter_mut() {
            visitor.visit_term(value);
        }
        visitor.visit_term(&mut self.body);
    }
}

impl<'a> Lambda<'a> {
    pub fn walk(&mut self, visitor: &mut impl Visitor) {
        visitor.visit_term(&mut self.body);
    }
}

impl<'a> Block<'a> {
    pub fn walk(&mut self, visitor: &mut impl Visitor) {
        for term in &mut self.body {
            visitor.visit_term(term);
        }
    }
}

impl<'a> Quote<'a> {
    pub fn walk(&mut self, _: &mut impl Visitor) {}
}

impl<'a> If<'a> {
    pub fn walk(&mut self, visitor: &mut impl Visitor) {
        visitor.visit_term(&mut self.cond);
        visitor.visit_term(&mut self.then);
        visitor.visit_term(&mut self.else_);
    }
}

impl<'a> Call<'a> {
    pub fn walk(&mut self, visitor: &mut impl Visitor) {
        visitor.visit_term(&mut self.func);
        for arg in &mut self.args {
            visitor.visit_term(arg);
        }
    }
}

impl<'a> Term<'a> {
    pub fn walk(&mut self, visitor: &mut impl Visitor) {
        match self {
            Term::Atom(atom) => visitor.visit_atom(atom),
            Term::TypeOf(type_of) => visitor.visit_type_of(type_of),
            Term::Vector(vector) => visitor.visit_vector(vector),
            Term::Cons(cons) => visitor.visit_cons(cons),
            Term::Nil(nil) => visitor.visit_nil(nil),
            Term::Head(head) => visitor.visit_head(head),
            Term::Tail(tail) => visitor.visit_tail(tail),
            Term::VectorIndex(vector_index) => visitor.visit_vector_index(vector_index),
            Term::VectorLen(vector_len) => visitor.visit_vector_len(vector_len),
            Term::VectorPush(vector_push) => visitor.visit_vector_push(vector_push),
            Term::Box(box_) => visitor.visit_box(box_),
            Term::Unbox(unbox) => visitor.visit_unbox(unbox),
            Term::CreateClosure(create_closure) => visitor.visit_create_closure(create_closure),
            Term::Binary(binary) => visitor.visit_binary(binary),
            Term::Number(number) => visitor.visit_number(number),
            Term::Str(str_) => visitor.visit_str(str_),
            Term::Bool(bool_) => visitor.visit_bool(bool_),
            Term::Variable(variable) => visitor.visit_variable(variable),
            Term::Let(let_) => visitor.visit_let(let_),
            Term::Lambda(lambda) => visitor.visit_lambda(lambda),
            Term::Block(block) => visitor.visit_block(block),
            Term::Quote(quote) => visitor.visit_quote(quote),
            Term::If(if_) => visitor.visit_if(if_),
            Term::Call(call) => visitor.visit_call(call),
            Term::IsNil(is_nil) => visitor.visit_is_nil(is_nil),
        }
    }
}

impl<'a> From<Atom> for Term<'a> {
    fn from(atom: Atom) -> Self {
        Term::Atom(atom)
    }
}

impl<'a> From<TypeOf<'a>> for Term<'a> {
    fn from(type_of: TypeOf<'a>) -> Self {
        Term::TypeOf(type_of)
    }
}

impl<'a> From<Vector<'a>> for Term<'a> {
    fn from(vector: Vector<'a>) -> Self {
        Term::Vector(vector)
    }
}

impl<'a> From<Cons<'a>> for Term<'a> {
    fn from(cons: Cons<'a>) -> Self {
        Term::Cons(cons)
    }
}

impl From<Nil> for Term<'_> {
    fn from(nil: Nil) -> Self {
        Term::Nil(nil)
    }
}

impl<'a> From<Head<'a>> for Term<'a> {
    fn from(head: Head<'a>) -> Self {
        Term::Head(head)
    }
}

impl<'a> From<Tail<'a>> for Term<'a> {
    fn from(tail: Tail<'a>) -> Self {
        Term::Tail(tail)
    }
}

impl<'a> From<IsNil<'a>> for Term<'a> {
    fn from(is_nil: IsNil<'a>) -> Self {
        Term::IsNil(is_nil)
    }
}

impl<'a> From<VectorIndex<'a>> for Term<'a> {
    fn from(vector_index: VectorIndex<'a>) -> Self {
        Term::VectorIndex(vector_index)
    }
}

impl<'a> From<VectorLen<'a>> for Term<'a> {
    fn from(vector_len: VectorLen<'a>) -> Self {
        Term::VectorLen(vector_len)
    }
}

impl<'a> From<VectorPush<'a>> for Term<'a> {
    fn from(vector_push: VectorPush<'a>) -> Self {
        Term::VectorPush(vector_push)
    }
}

impl<'a> From<BoxTerm<'a>> for Term<'a> {
    fn from(box_term: BoxTerm<'a>) -> Self {
        Term::Box(box_term)
    }
}

impl<'a> From<UnboxTerm<'a>> for Term<'a> {
    fn from(unbox_term: UnboxTerm<'a>) -> Self {
        Term::Unbox(unbox_term)
    }
}

impl<'a> From<Function<'a>> for Term<'a> {
    fn from(create_closure: Function<'a>) -> Self {
        Term::CreateClosure(create_closure)
    }
}

impl<'a> From<Binary<'a>> for Term<'a> {
    fn from(binary: Binary<'a>) -> Self {
        Term::Binary(binary)
    }
}

impl From<Number> for Term<'_> {
    fn from(number: Number) -> Self {
        Term::Number(number)
    }
}

impl<'a> From<Str<'a>> for Term<'a> {
    fn from(str: Str<'a>) -> Self {
        Term::Str(str)
    }
}

impl From<Bool> for Term<'_> {
    fn from(bool: Bool) -> Self {
        Term::Bool(bool)
    }
}

impl<'a> From<Variable> for Term<'a> {
    fn from(variable: Variable) -> Self {
        Term::Variable(variable)
    }
}

impl<'a> From<Let<'a>> for Term<'a> {
    fn from(let_: Let<'a>) -> Self {
        Term::Let(let_)
    }
}

impl<'a> From<Lambda<'a>> for Term<'a> {
    fn from(lambda: Lambda<'a>) -> Self {
        Term::Lambda(lambda)
    }
}

impl<'a> From<Block<'a>> for Term<'a> {
    fn from(block: Block<'a>) -> Self {
        Term::Block(block)
    }
}

impl<'a> From<Quote<'a>> for Term<'a> {
    fn from(quote: Quote<'a>) -> Self {
        Term::Quote(quote)
    }
}

impl<'a> From<If<'a>> for Term<'a> {
    fn from(if_: If<'a>) -> Self {
        Term::If(if_)
    }
}

impl<'a> From<Call<'a>> for Term<'a> {
    fn from(call: Call<'a>) -> Self {
        Term::Call(call)
    }
}

pub enum SExpr {
    Id(String),
    List(Vec<SExpr>),
}

impl SExpr {
    pub fn label(str: &str) -> Self {
        SExpr::Id(str.to_string())
    }

    pub fn with<T: Show>(self, expr: &T) -> SExpr {
        let mut list = match self {
            SExpr::Id(id) => vec![SExpr::Id(id)],
            SExpr::List(list) => list,
        };
        list.push(expr.show());
        SExpr::List(list)
    }

    pub fn with_vec<T: Show>(self, exprs: &[T]) -> SExpr {
        let mut list = match self {
            SExpr::Id(id) => vec![SExpr::Id(id)],
            SExpr::List(list) => list,
        };
        list.extend(exprs.iter().map(|e| e.show()));
        SExpr::List(list)
    }

    pub fn within(self, expr: SExpr) -> SExpr {
        let mut list = match self {
            SExpr::Id(id) => vec![SExpr::Id(id)],
            SExpr::List(list) => list,
        };
        list.push(expr);
        SExpr::List(list)
    }

    pub fn extend(self, exprs: Vec<SExpr>) -> SExpr {
        let mut list = match self {
            SExpr::Id(id) => vec![SExpr::Id(id)],
            SExpr::List(list) => list,
        };
        list.extend(exprs);
        SExpr::List(list)
    }

    pub fn width(&self) -> usize {
        match self {
            SExpr::Id(id) => id.len(),
            SExpr::List(list) => list.iter().map(|e| e.width()).sum(),
        }
    }

    pub fn print(&self, fmt: &mut fmt::Formatter, indent: usize) -> fmt::Result {
        write!(fmt, "{:indent$}", "", indent = indent)?;
        match self {
            SExpr::Id(id) => write!(fmt, "{}", id),
            SExpr::List(list) => {
                if self.width() + indent < 30 {
                    write!(fmt, "(")?;
                    if !list.is_empty() {
                        list[0].print(fmt, 0)?;
                        for e in &list[1..] {
                            write!(fmt, " ")?;
                            e.print(fmt, 0)?;
                        }
                    }
                    write!(fmt, ")")
                } else {
                    write!(fmt, "(")?;
                    if !list.is_empty() {
                        list[0].print(fmt, 0)?;
                        for e in &list[1..] {
                            writeln!(fmt)?;
                            e.print(fmt, indent + 1)?;
                        }
                    }
                    write!(fmt, ")")
                }
            }
        }
    }
}

impl Display for SExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.print(f, 0)
    }
}

pub trait Show {
    fn show(&self) -> SExpr;
}

impl<'a> Show for TypeOf<'a> {
    fn show(&self) -> SExpr {
        SExpr::label("typeof").with(&self.expr)
    }
}

impl Show for Atom {
    fn show(&self) -> SExpr {
        SExpr::label(&format!(":{}", self.name.debug_name))
    }
}

impl<'a> Show for Vector<'a> {
    fn show(&self) -> SExpr {
        SExpr::label("vector").with_vec(&self.elements)
    }
}

impl<'a> Show for Cons<'a> {
    fn show(&self) -> SExpr {
        SExpr::label("cons").with(&self.head).with(&self.tail)
    }
}

impl Show for Nil {
    fn show(&self) -> SExpr {
        SExpr::label("nil")
    }
}

impl<'a> Show for Head<'a> {
    fn show(&self) -> SExpr {
        SExpr::label("head").with(&self.list)
    }
}

impl<'a> Show for Tail<'a> {
    fn show(&self) -> SExpr {
        SExpr::label("tail").with(&self.list)
    }
}

impl<'a> Show for IsNil<'a> {
    fn show(&self) -> SExpr {
        SExpr::label("is-nil").with(&self.list)
    }
}

impl<'a> Show for VectorIndex<'a> {
    fn show(&self) -> SExpr {
        SExpr::label("vector-index")
            .with(&self.vector)
            .with(&self.index)
    }
}

impl<'a> Show for VectorLen<'a> {
    fn show(&self) -> SExpr {
        SExpr::label("vector-len").with(&self.vector)
    }
}

impl<'a> Show for VectorPush<'a> {
    fn show(&self) -> SExpr {
        SExpr::label("vector-push")
            .with(&self.vector)
            .with(&self.element)
    }
}

impl<'a> Show for BoxTerm<'a> {
    fn show(&self) -> SExpr {
        SExpr::label("box").with(&self.term)
    }
}

impl<'a> Show for UnboxTerm<'a> {
    fn show(&self) -> SExpr {
        SExpr::label("unbox").with(&self.term)
    }
}

impl<'a> Show for Function<'a> {
    fn show(&self) -> SExpr {
        SExpr::label("function")
            .within(SExpr::List(self.env.iter().map(|a| a.show()).collect()))
            .within(SExpr::List(self.params.iter().map(|a| a.show()).collect()))
            .with(&self.body)
    }
}

impl Show for &str {
    fn show(&self) -> SExpr {
        SExpr::label(self)
    }
}

impl Show for Symbol {
    fn show(&self) -> SExpr {
        SExpr::label(&self.debug_name)
    }
}

impl<'a> Show for Binary<'a> {
    fn show(&self) -> SExpr {
        SExpr::label(&format!("{:?}", self.op))
            .with(&self.left)
            .with(&self.right)
    }
}

impl Show for Number {
    fn show(&self) -> SExpr {
        SExpr::label(&format!("{}", self.value))
    }
}

impl<'a> Show for Str<'a> {
    fn show(&self) -> SExpr {
        SExpr::label(&format!("{:?}", self.value))
    }
}

impl Show for Bool {
    fn show(&self) -> SExpr {
        SExpr::label(&format!("{}", self.value))
    }
}

impl Show for Variable {
    fn show(&self) -> SExpr {
        match self {
            Variable::Local { name, index } => {
                SExpr::label(&format!("{}~{}", name.debug_name, index))
            }
            Variable::Global { name } => SExpr::label(&format!("#{}", name.debug_name)),
            Variable::Env { name, index } => {
                SExpr::label(&format!("!{}~{}", name.debug_name, index))
            }
        }
    }
}

impl<'a> Show for Let<'a> {
    fn show(&self) -> SExpr {
        let bindings = self
            .bindings
            .iter()
            .map(|(name, term)| SExpr::List(vec![name.show(), term.show()]));

        SExpr::label("let")
            .extend(bindings.collect())
            .with(&self.body)
    }
}

impl<'a> Show for Lambda<'a> {
    fn show(&self) -> SExpr {
        SExpr::label("lambda")
            .within(SExpr::List(self.args.iter().map(Show::show).collect_vec()))
            .with(&self.body)
    }
}

impl<'a> Show for Block<'a> {
    fn show(&self) -> SExpr {
        SExpr::label("block").with_vec(&self.body)
    }
}

impl<'a> Show for Quote<'a> {
    fn show(&self) -> SExpr {
        SExpr::label("quote").with(&self.value)
    }
}

impl Show for u64 {
    fn show(&self) -> SExpr {
        SExpr::label(&format!("{}", self))
    }
}

impl<T: Show> Show for Vec<T> {
    fn show(&self) -> SExpr {
        SExpr::List(self.iter().map(|a| a.show()).collect_vec())
    }
}

impl<'a> Show for Expr<'a> {
    fn show(&self) -> SExpr {
        match &self.data {
            ExprKind::Atom(a) => a.show(),
            ExprKind::Identifier(a) => a.show(),
            ExprKind::List(a) => a.show(),
            ExprKind::Number(a) => a.show(),
            ExprKind::String(a) => a.show(),
        }
    }
}

impl<'a> Show for If<'a> {
    fn show(&self) -> SExpr {
        SExpr::label("if")
            .with(&self.cond)
            .with(&self.then)
            .with(&self.else_)
    }
}

impl<'a> Show for Call<'a> {
    fn show(&self) -> SExpr {
        SExpr::label("call").with(&self.func).with_vec(&self.args)
    }
}

impl<T: Show> Show for Box<T> {
    fn show(&self) -> SExpr {
        (**self).show()
    }
}

impl<'a> Show for Term<'a> {
    fn show(&self) -> SExpr {
        match self {
            Term::Atom(a) => a.show(),
            Term::TypeOf(a) => a.show(),
            Term::Vector(a) => a.show(),
            Term::Cons(a) => a.show(),
            Term::Nil(a) => a.show(),
            Term::Head(a) => a.show(),
            Term::Tail(a) => a.show(),
            Term::IsNil(a) => a.show(),
            Term::VectorIndex(a) => a.show(),
            Term::VectorLen(a) => a.show(),
            Term::VectorPush(a) => a.show(),
            Term::Box(a) => a.show(),
            Term::Unbox(a) => a.show(),
            Term::CreateClosure(a) => a.show(),
            Term::Binary(a) => a.show(),
            Term::Number(a) => a.show(),
            Term::Str(a) => a.show(),
            Term::Bool(a) => a.show(),
            Term::Variable(a) => a.show(),
            Term::Let(a) => a.show(),
            Term::Lambda(a) => a.show(),
            Term::Block(a) => a.show(),
            Term::Quote(a) => a.show(),
            Term::If(a) => a.show(),
            Term::Call(a) => a.show(),
        }
    }
}
