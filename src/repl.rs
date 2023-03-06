use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;

use crate::{
    codegen::{execution, Codegen},
    runtime::ValueRef,
};

pub fn run() {
    unsafe {
        Codegen::install_execution_targets();
    }

    let mut rl = DefaultEditor::new().expect("cannot create a repl");

    loop {
        let readline = rl.readline("> ");

        match readline {
            Ok(line) => {
                rl.add_history_entry(line.as_str())
                    .expect("cannot add to the history");

                let parsed = crate::Parser::new().parse(&line);

                if let Ok(term) = parsed {
                    let expr = crate::specialized::Term::specialize(term);
                    let converted = crate::closure::convert(expr);

                    unsafe {
                        let mut codegen = Codegen::try_new()
                            .unwrap()
                            .install_error_handling()
                            .install_primitives();

                        codegen.compile_main(converted);
                        codegen.dump_module();
                        codegen.verify_module().unwrap_or_else(|error| {
                            for line in error.split("\n") {
                                println!("[error*] {}", line);
                            }

                            panic!("Module verification failed")
                        });

                        let engine = execution::ExecutionEngine::try_new(codegen.module)
                            .unwrap()
                            .add_primitive_symbols(&codegen.symbols);

                        let f: extern "C" fn() -> ValueRef =
                            std::mem::transmute(engine.get_function_address("main"));

                        println!("{}", f());
                    }
                } else {
                    println!("Cannot parse")
                }
            }
            Err(ReadlineError::Interrupted) => {
                println!("Bye bye...");
                break;
            }
            Err(ReadlineError::Eof) => {
                println!("CTRL-D");
                break;
            }
            Err(err) => {
                println!("Error: {err:?}");
                break;
            }
        }
    }
}
