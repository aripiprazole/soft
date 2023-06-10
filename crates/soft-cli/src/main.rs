//! This is a couple project with my beloved girlfriend @algebraic_gabii. I love her so much :> I
//! hope that one day she will be a confident person that will be able to do anything she wants.
//! This is just a love letter for her.
//!
//! But ok, let's talk about the project. This is a compiler for a simple LISP that is compiled
//! using cranelift and that will store the code in a "database" so we can restart from a file
//! instead of a bunch of files.

use std::{env, process::exit};

use soft_compiler::backend::llvm::codegen::Options;
use soft_compiler::backend::Backend;
use soft_compiler::backend::Runnable;
use soft_compiler::specialize::tree::Show;

use soft_compiler::{backend::llvm, parser::parse};

fn main() {
    // The CLI only takes one expression and then executes it. The first thing that you're going to
    // use is probably an injection of a function that will be used to start the program.
    match env::args().collect::<Vec<_>>().as_slice() {
        [_cwd, code] => {
            let parsed = parse(code).expect("oh no");

            let mut terms: Vec<_> = parsed.into_iter().map(|x| x.to_term()).collect();

            for term in &mut terms {
                term.closure_convert();
                println!("{}", term.show())
            }

            let opt = Options::default();
            let llvm = llvm::Context::new(&opt);
            let result = llvm.compile(terms);

            result.unwrap().run();
        }
        _ => {
            println!("[err] expected just one string to run.");
            exit(1)
        }
    }
}
