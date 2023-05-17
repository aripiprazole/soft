//! This module describes a tree for a specialized version of the soft s-expression language. It's
//! used to represent the AST of the language in a easier way by classifying structures that can be
//! optimized by compilation.

use std::fmt::{Formatter, Display};

use crate::syntax::Expr;

/// This enum represents if a lambda was lifted or not to the global scope.
#[derive(Clone, Copy, Debug)]
pub enum Lifted {
    Yes,
    No,
}

/// Represents a lambda function that is used to create closures.
#[derive(Debug, Clone)]
pub struct LambdaNode {
    pub args: Vec<String>,
    pub body: Vec<Term>,
    pub lifted: Lifted,
}

/// Represents an application of a function or closure by a number of arguments.
#[derive(Debug, Clone)]
pub struct ApplicationNode {
    pub function: Box<Term>,
    pub arguments: Vec<Term>,
}

/// Represents a type of reference of a variable node. It can be local, global or a closure.
///
/// - Local   : Reference to a local argument
/// - Global  : reference to the enviroment argument
/// - Closure : reference to a closure environment
///
#[derive(Clone, Copy, Debug)]
pub enum ReferenceType {
    Local,
    Global,
    Closure,
}

/// Represents a variable that is used to reference a value.
#[derive(Debug, Clone)]
pub struct VarNode {
    pub name: String,
    pub reference: ReferenceType,
}

/// Represents a literal that is used to represent a number, string or nil.
#[derive(Debug, Clone)]
pub enum LiteralNode {
    Number(u64),
    String(String),
    Nil,
}

#[derive(Clone, Copy, Debug)]
/// Represents if a set! is a macro or not.
pub enum IsMacro {
    Yes,
    No,
}

/// Represents a set! expression that is used to set a variable globally.
#[derive(Debug, Clone)]
pub struct SetNode {
    pub name: String,
    pub value: Box<Term>,
    pub is_macro: IsMacro,
}

/// Represents a quote that is used to prevent evaluation of a term.
#[derive(Debug, Clone)]
pub struct QuoteNode {
    pub value: Box<Expr>,
}

/// Represents a cons cell that is used to represent a list.
#[derive(Debug, Clone)]
pub struct ConsNode {
    pub head: Box<Term>,
    pub tail: Box<Term>,
}

/// Represents an if expression that return it's value just like a ternary.
#[derive(Debug, Clone)]
pub struct IfNode {
    pub condition: Box<Term>,
    pub then: Box<Term>,
    pub else_: Box<Term>,
}

/// Represents an atom that is used to represent a symbol. O(1) comparison
#[derive(Debug, Clone)]
pub struct AtomNode {
    pub name: String,
}

/// This tree is both used for a lot of optimization passes before turning into a low level
/// imperative IR.
#[derive(Debug, Clone)]
pub enum Term {
    Atom(AtomNode),
    Lambda(LambdaNode),
    Application(ApplicationNode),
    Literal(LiteralNode),
    Variable(VarNode),
    Set(SetNode),
    Quote(QuoteNode),
    Cons(ConsNode),
    If(IfNode),
}

impl Term {
    pub fn lambda(args: Vec<String>, body: Vec<Term>, lifted: Lifted) -> Self {
        Self::Lambda(LambdaNode { args, body, lifted })
    }

    pub fn application(function: Term, arguments: Vec<Term>) -> Self {
        Self::Application(ApplicationNode {
            function: Box::new(function),
            arguments,
        })
    }

    pub fn literal(literal: LiteralNode) -> Self {
        Self::Literal(literal)
    }

    pub fn variable(name: String, reference: ReferenceType) -> Self {
        Self::Variable(VarNode { name, reference })
    }

    pub fn atom(name: String) -> Self {
        Self::Atom(AtomNode { name })
    }

    pub fn set(name: String, value: Term, is_macro: IsMacro) -> Self {
        Self::Set(SetNode {
            name,
            value: Box::new(value),
            is_macro,
        })
    }

    pub fn quote(value: Expr) -> Self {
        Self::Quote(QuoteNode {
            value: Box::new(value),
        })
    }

    pub fn cons(head: Term, tail: Term) -> Self {
        Self::Cons(ConsNode {
            head: Box::new(head),
            tail: Box::new(tail),
        })
    }

    pub fn cond(condition: Term, then: Term, else_: Term) -> Self {
        Self::If(IfNode {
            condition: Box::new(condition),
            then: Box::new(then),
            else_: Box::new(else_),
        })
    }
}

impl Display for AtomNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, ":{}", self.name)
    }
}

impl Display for LambdaNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "(lambda (")?;
        for arg in &self.args {
            write!(f, "{} ", arg)?;
        }
        write!(f, ") ")?;
        for term in &self.body {
            write!(f, "{} ", term)?;
        }
        write!(f, ")")
    }
}

impl Display for ApplicationNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "(")?;
        write!(f, "{} ", self.function)?;
        for arg in &self.arguments {
            write!(f, "{} ", arg)?;
        }
        write!(f, ")")
    }
}