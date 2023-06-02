use crate::specialize::tree::Term;

/// Statement representation for `MIR`. It does transforms the code into a sequential and imperative
/// representation, to make easier to compile to backends such as `LLVM`, `cranelift`, etc.
///
/// The goal is running a 'codegen' step, and transforming into control-flow graph into a further
/// step, because its going to be easier.
///
/// MIR -> CFG => Backend Specific Code
pub enum Stmt<'a> {
    /// Assignes a variable with name [String] and the value [Term].
    ///
    /// ```txt
    /// %x = 10;
    /// ```
    Assign(String, Term<'a>),

    /// If instruction, takes a conditions and two blocks of code.
    ///
    /// ```txt
    /// if %x:
    ///     %y;
    /// else:
    ///     %z;
    /// ```
    If(Term<'a>, Vec<Stmt<'a>>, Vec<Stmt<'a>>),

    /// Defines a variable in the current scope.
    Def(String, Option<Term<'a>>),

    /// Evaluates a term [Term].
    Term(Term<'a>),
}
