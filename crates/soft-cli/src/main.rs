//! This is a couple project with my beloved girlfriend @algebraic_gabii. I love her so much :> I
//! hope that one day she will be a confident person that will be able to do anything she wants.
//! This is just a love letter for her.
//!
//! But ok, let's talk about the project. This is a compiler for a simple LISP that is compiled
//! using cranelift and that will store the code in a "database" so we can restart from a file
//! instead of a bunch of files.

use std::{env, process::exit};

use soft_compiler::specialize::closure::ClosureConvert;
use soft_compiler::{parser::parse, specialize::specialize};

fn main() {
    /// The CLI only takes one expression and then executes it. The first thing that you're going to
    /// use is probably an injection of a function that will be used to start the program.
    match env::args().collect().as_slice() {
        [_cwd, code] => {
            let parsed = parse(code).unwrap();
            let mut specialized = specialize(parsed[0].clone());
            specialized.closure_convert();
        }
        _ => {
            println!("[err] expected just one string to run.");
            exit(1)
        }
    }
}
