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

                match eval_line(line) {
                    Ok(value_ref) => {
                        println!(": {value_ref}")
                    }
                    Err(err) => {
                        println!("[error] Uncaught error when evaluating:");
                        println!("~ {err}")
                    }
                }
            }
            Err(ReadlineError::Interrupted) => {
                println!("[exit] Bye bye... 👋");
                break;
            }
            Err(ReadlineError::Eof) => {
                println!("[exit] CTRL-D");
                break;
            }
            Err(err) => {
                println!("[error] Uncaught I/O error: {err:?}");
                break;
            }
        }
    }
}

fn eval_line(line: String) -> Result<ValueRef, String> {
    let value = crate::Parser::new()
        .parse(&line)
        .map_err(|err| err.to_string())?;

    let term = value.specialize().map_err(|e| {
        format!(
            "[error] Could not specialize expression on code [{}:{}:{}]: {}",
            e.r_source_file, e.r_source_line, e.r_source_column, e.message
        )
    })?;

    let converted = term.convert();

    unsafe {
        let mut codegen = Codegen::try_new()
            .unwrap()
            .install_error_handling()
            .install_primitives();

        codegen.compile_main(converted).unwrap();
        codegen.dump_module();
        codegen.verify_module().unwrap_or_else(|error| {
            for line in error.split("\n") {
                println!("[error*] {}", line);
            }

            panic!("Module verification failed")
        });

        let engine = execution::ExecutionEngine::try_new(codegen.module)
            .unwrap()
            .add_primitive_symbols(&codegen.environment);

        let f: extern "C" fn() -> ValueRef =
            std::mem::transmute(engine.get_function_address("main"));

        Ok(f())
    }
}
