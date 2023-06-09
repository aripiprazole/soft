//! This is the main module that compiles a sequence of s-expressions into a low level control graph
//! structure. The order that the things happens is not always linear because LISP is a circular
//! language, so the compilation of some of the things happen with the help with a global state
//! defined in the runtime.

pub mod cranelift;
pub mod expr;
pub mod llvm;
pub mod location;
pub mod mir;
pub mod parser;
pub mod specialize;
