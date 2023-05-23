use std::hash::Hash;

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
}

/// Symbol is a struct that makes string comparisons O(1) by comparing it's memoized hash, instead
/// of it's names, but still uses the names to better error handling and debug.
#[derive(Clone, Eq)]
pub struct Symbol<'a> {
    pub debug_name: &'a str,
    pub hash: usize,
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
}

pub enum VariableKind<'a> {
    /// Global variable in the environment, indexed by [Symbol], it can be accessed using the hash
    /// property in the [Symbol] struct.
    ///
    /// The global variable isn't known if it exists on the environment, so it can cause undefined
    /// behavior in some cases, differently from [VariableKind::Local] variables, that is certified
    /// it does exists when it's accessed.
    Global(Symbol<'a>),

    /// Local variable in the local scope, indexed by [usize].
    Local(usize, &'a str),

    /// Reference to lambda-lifted variables, that are in the global scope currently, but were
    /// closures.
    ///
    /// TODO: document, and examples
    Lifted(usize),
}

/// TODO: while, TCO
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
    Let(Vec<(Symbol<'a>, Term<'a>)>),

    /// set!
    ///
    /// A set statement, that sets a variable in the global scope.
    Set(Symbol<'a>, Box<Term<'a>>, IsMacro),

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
    If(Box<Expr<'a>>, Box<Expr<'a>>, Option<Box<Expr<'a>>>),

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

/// An [TermKind] with a range of positions in the source code. It's used in order to make better
/// error messages.
pub type Term<'a> = Spanned<TermKind<'a>>;

pub struct Definition<'a> {
    pub variadic_parameter: Option<Symbol<'a>>,
    pub parameters: Vec<Symbol<'a>>,
    pub body: Vec<Term<'a>>,
}

pub struct Function<'a> {
    pub name: Symbol<'a>,

    /// Definitions can hold the lambda-lifted expressions and the function expression. The first
    /// item on the [definitions] vec, is the function body's definition.
    pub definitions: Vec<Definition<'a>>,
}

// An expression that was lifted to the global scope. It's used for the REPL.
pub struct Expression<'a> {
    pub expr: Term<'a>,
    pub defintions: Vec<Definition<'a>>,
}
