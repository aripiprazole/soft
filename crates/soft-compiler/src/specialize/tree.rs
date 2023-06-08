use std::{fmt::Display, hash::Hash};

use crate::{location::Spanned, parser::syntax::Expr};

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

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum IsMacro {
    Yes,
    No,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum IsLifted {
    Yes,
    No,
}

#[derive(Debug, Clone)]
pub enum PrimKind<'a> {
    /// Gets the type of an expression and returns it as an atom.
    /// e.g:
    ///
    /// ```lisp
    /// (type-of 2)
    /// ```
    TypeOf(Box<Term<'a>>),

    /// A vector expression that is surrounded by parenthesis, and starts with a function-call.
    ///
    /// E.g:
    /// ```lisp
    /// (vec! 1 2 3 4)
    /// ```
    Vec(Vec<Term<'a>>),

    /// A linked list cons cell. It adds an element at the start of a linked list
    Cons(Box<Term<'a>>, Box<Term<'a>>),

    /// An empty linked list.
    Nil,

    /// Gets the first element of a cons cell, otherwise it throws a condition.
    Head(Box<Term<'a>>),

    /// Gets the second element of a cons cell, otherwise it throws a condition.
    Tail(Box<Term<'a>>),

    /// Indexes a vector using a number.
    VecIndex(Box<Term<'a>>, Box<Term<'a>>),

    /// Gets the length of a vector.
    VecLength(Box<Term<'a>>),

    /// Sets the index of a vector.
    VecSet(Box<Term<'a>>, Box<Term<'a>>, Box<Term<'a>>),

    /// Creates a boxed value.
    Box(Box<Term<'a>>),

    /// Copies the value from the box.
    Unbox(Box<Term<'a>>),

    /// Sets the value inside of a box.
    BoxSet(Box<Term<'a>>, Box<Term<'a>>),

    /// Gets the environment of a closure.
    GetEnv(Symbol<'a>),

    // Creates a closure from a function and a list of arguments.
    CreateClosure(Box<Term<'a>>, Vec<(Symbol<'a>, Term<'a>)>),
}

impl<'a> Display for PrimKind<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PrimKind::TypeOf(expr) => write!(f, "(type-of {})", expr),
            PrimKind::Vec(exprs) => {
                write!(f, "(vec!")?;
                for expr in exprs {
                    write!(f, " {}", expr)?;
                }
                write!(f, ")")
            }
            PrimKind::Cons(head, tail) => write!(f, "(cons {} {})", head, tail),
            PrimKind::Nil => write!(f, "nil"),
            PrimKind::Head(expr) => write!(f, "(head {})", expr),
            PrimKind::Tail(expr) => write!(f, "(tail {})", expr),
            PrimKind::VecIndex(vec, index) => write!(f, "(vec-index {} {})", vec, index),
            PrimKind::VecLength(vec) => write!(f, "(vec-length {})", vec),
            PrimKind::VecSet(vec, index, value) => {
                write!(f, "(vec-set! {} {} {})", vec, index, value)
            }
            PrimKind::Box(expr) => write!(f, "(box {})", expr),
            PrimKind::Unbox(expr) => write!(f, "(unbox {})", expr),
            PrimKind::BoxSet(expr, value) => write!(f, "(box-set! {} {})", expr, value),
            PrimKind::GetEnv(expr) => write!(f, "(get-env {})", expr),
            PrimKind::CreateClosure(func, args) => {
                write!(f, "(create-closure {} ", func)?;
                if !args.is_empty() {
                    write!(f, "({} {})", args[0].0, args[0].1)?;
                    for (name, arg) in &args[1..] {
                        write!(f, " ({} {})", name, arg)?;
                    }
                }
                write!(f, ")")
            }
        }
    }
}

/// Symbol is a struct that makes string comparisons O(1) by comparing it's memoized hash, instead
/// of it's names, but still uses the names to better error handling and debug.
#[derive(Clone, Eq, Debug)]
pub struct Symbol<'a> {
    debug_name: &'a str,
    hash: usize,
}

impl<'a> PartialEq for Symbol<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.hash == other.hash
    }
}

/// Reuses the hash symbol's generated hash
impl<'a> Hash for Symbol<'a> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write_usize(self.hash);
    }
}

impl<'a> Symbol<'a> {
    /// Creates a new symbol, and generates a hash for it using [fxhash].
    pub fn new(debug_name: &'a str) -> Self {
        Self {
            debug_name,
            hash: fxhash::hash(debug_name),
        }
    }

    pub fn name(&self) -> &'a str {
        self.debug_name
    }
}

#[derive(Debug, Clone)]
pub enum VariableKind<'a> {
    /// Global variable in the environment, indexed by [Symbol], it can be accessed using the hash
    /// property in the [Symbol] struct.
    ///
    /// The global variable isn't known if it exists on the environment, so it can cause undefined
    /// behavior in some cases, differently from [VariableKind::Local] variables, that is certified
    /// it does exists when it's accessed.
    Global(Symbol<'a>),

    /// Local variable in the local scope, indexed by [usize].
    Local(usize, Symbol<'a>),
}

/// TODO: while, TCO
#[derive(Debug, Clone)]
pub enum TermKind<'a> {
    // S Expressions
    /// An atom is a globally available constant that is defined by it's name that is O(1) for
    /// comparison.
    ///
    /// E.g:
    /// ```lisp
    /// 'some, 'name, 'some, 'atom
    /// ```
    Atom(&'a str),

    /// An unsigned number literal of 60 bytes.
    Number(u64),

    /// A string literal. It's represented as a UTF-8 array that cannot be indexed.
    String(&'a str),

    /// Boolean literal (special case of Atom for :true and :false)    
    Bool(bool),

    // Language Constructs
    /// An identifier is a name that is used to reference a variable or a function.
    Variable(VariableKind<'a>),

    /// let!
    ///
    /// A let statement, that sets a variable in the local scope.
    Let(Vec<(Symbol<'a>, Term<'a>)>, Box<Term<'a>>),

    /// set!
    ///
    /// A set statement, that sets a variable in the global scope.
    Set(Symbol<'a>, Box<Expr<'a>>, Box<Term<'a>>, IsMacro),

    /// lambda!
    ///
    /// A lambda expression, that creates a local closure, and [IsLifted] marks if the function
    /// is lambda-lifted or closure-converted.
    Lambda(Definition<'a>, IsLifted),

    /// block!
    ///
    /// A statement block.
    Block(Vec<Term<'a>>),

    /// quote!
    ///
    /// A quote expression.
    Quote(Box<Expr<'a>>),

    /// if!
    ///
    /// A ternary conditional expression. If the scrutinee express a value of truth then it executes
    /// the first branch (the second argument on the variant), otherwise, it executes the second one
    /// (if the second one is None then it returns nil)
    If(Box<Term<'a>>, Box<Term<'a>>, Box<Term<'a>>),

    /// A binary or unary operation iterated expression. It can take an arbitrary number of
    /// arguments.
    Operation(OperationKind, Vec<Term<'a>>),

    // Runtime Calls
    /// Function call with a term on the function side. It`s only permitted during the unlifted
    /// phase, after the lambda-lifting, all of the Call function terms should be a Variable.
    Call(Box<Term<'a>>, Vec<Term<'a>>),

    // A primitive function call that we are going to optimize.
    Prim(PrimKind<'a>),
}

impl<'a> Display for Symbol<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.debug_name)
    }
}

impl<'a> Display for Term<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.data)
    }
}

impl Display for OperationKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OperationKind::Add => write!(f, "+"),
            OperationKind::Sub => write!(f, "-"),
            OperationKind::Mul => write!(f, "*"),
            OperationKind::Div => write!(f, "/"),
            OperationKind::Mod => write!(f, "%"),
            OperationKind::Eql => write!(f, "="),
            OperationKind::Neq => write!(f, "!="),
            OperationKind::Lte => write!(f, "<"),
            OperationKind::Ltn => write!(f, "<="),
            OperationKind::Gte => write!(f, ">"),
            OperationKind::Gtn => write!(f, ">="),
            OperationKind::And => write!(f, "&"),
            OperationKind::Or => write!(f, "|"),
            OperationKind::Not => write!(f, "!"),
            OperationKind::Xor => write!(f, "^"),
            OperationKind::Shl => write!(f, "shl"),
            OperationKind::Shr => write!(f, "shr"),
            OperationKind::LAnd => write!(f, "and"),
            OperationKind::LOr => write!(f, "or"),
        }
    }
}

impl<'a> Display for TermKind<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TermKind::Atom(name) => write!(f, "'{}", name),
            TermKind::Number(n) => write!(f, "{}", n),
            TermKind::String(s) => write!(f, "\"{}\"", s),
            TermKind::Bool(b) => write!(f, "{}", b),
            TermKind::Variable(v) => match v {
                VariableKind::Local(idx, name) => write!(f, "{}~{idx}", name),
                VariableKind::Global(name) => write!(f, "#{}", name),
            },
            TermKind::Let(definitions, body) => {
                write!(f, "(let! (")?;
                for (name, value) in definitions {
                    write!(f, "({} {})", name, value)?;
                }
                write!(f, ") {})", body)
            }
            TermKind::Set(name, ast, value, is_macro) => {
                write!(
                    f,
                    "(set! {} (vec! {} {} {}))",
                    name,
                    ast,
                    value,
                    match is_macro {
                        IsMacro::Yes => "true",
                        IsMacro::No => "nil",
                    }
                )
            }
            TermKind::Lambda(def, is_lifted) => {
                write!(
                    f,
                    "(lambda{} (",
                    match is_lifted {
                        IsLifted::Yes => "!",
                        IsLifted::No => "",
                    }
                )?;
                if !def.parameters.is_empty() {
                    write!(f, "{}", def.parameters[0])?;
                    for parameter in &def.parameters[1..] {
                        write!(f, " {}", parameter)?;
                    }
                }
                write!(f, ") {})", def.body)
            }
            TermKind::Block(expressions) => {
                write!(f, "(block!")?;
                for expression in expressions {
                    write!(f, " {}", expression)?;
                }
                write!(f, ")")
            }
            TermKind::Quote(expr) => write!(f, "(quote! {})", expr),
            TermKind::If(scrutinee, then, else_) => {
                write!(f, "(if! {} {} {})", scrutinee, then, else_)
            }
            TermKind::Operation(operation, args) => {
                write!(f, "({}", operation)?;
                for arg in args {
                    write!(f, " {}", arg)?;
                }
                write!(f, ")")
            }
            TermKind::Call(function, args) => {
                write!(f, "({}", function)?;
                for arg in args {
                    write!(f, " {}", arg)?;
                }
                write!(f, ")")
            }
            TermKind::Prim(prim) => write!(f, "{}", prim),
        }
    }
}

/// An [TermKind] with a range of positions in the source code. It's used in order to make better
/// error messages.
pub type Term<'a> = Spanned<TermKind<'a>>;

#[derive(Debug, Clone)]
pub struct Definition<'a> {
    pub is_variadic: bool,
    pub parameters: Vec<Symbol<'a>>,
    pub body: Box<Term<'a>>,
}
