use std::fmt::Debug;

use crate::specialize::tree::Term;

use super::Instr;

/// Statement representation for `MIR`. It does transforms the code into a sequential and imperative
/// representation, to make easier to compile to backends such as `LLVM`, `cranelift`, etc.
///
/// The goal is running a 'codegen' step, and transforming into control-flow graph into a further
/// step, because its going to be easier.
///
/// MIR -> CFG => Backend Specific Code
#[derive(Clone)]
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

/// Defines that for [Stmt], the expression representation is [Term].
impl<'a> Instr for Stmt<'a> {
    type Term = Term<'a>;
}

impl<'a> Stmt<'a> {
    // Boilerplate functions
    pub fn assign(name: &str, value: Term<'a>) -> Self {
        Self::Assign(name.into(), value)
    }

    pub fn cond(cond: Term<'a>, then: Vec<Stmt<'a>>, otherwise: Vec<Stmt<'a>>) -> Self {
        Self::If(cond, then, otherwise)
    }

    pub fn set(name: &str, value: Term<'a>) -> Self {
        Self::Def(name.into(), Some(value))
    }

    pub fn def(name: &str) -> Self {
        Self::Def(name.into(), None)
    }

    pub fn eval(term: Term<'a>) -> Self {
        Self::Term(term)
    }

    fn render(&self, f: &mut std::fmt::Formatter<'_>, tab: &str) -> std::fmt::Result {
        write!(f, "{tab}")?;

        match self {
            Stmt::Assign(name, value) => {
                writeln!(f, "%{name} := {value:#?}")?;
            }
            Stmt::If(cond, then, otherwise) => {
                writeln!(f, "if {cond}:")?;
                for stmt in then {
                    stmt.render(f, &format!("{tab}    "))?;
                }
                if !otherwise.is_empty() {
                    writeln!(f, "else:")?;
                    for stmt in otherwise {
                        stmt.render(f, &format!("{tab}    "))?;
                    }
                }
            }
            Stmt::Def(name, Some(value)) => {
                writeln!(f, "%{name} = {value} ; def with value")?;
            }
            Stmt::Def(name, None) => {
                writeln!(f, "%{name} : soft_object ; def with value")?;
            }
            Stmt::Term(term) => {
                writeln!(f, "{term:#?}")?;
            }
        };

        Ok(())
    }
}

impl Debug for Stmt<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.render(f, Default::default())
    }
}
