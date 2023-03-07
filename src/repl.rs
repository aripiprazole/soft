use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;

use crate::{
    codegen::{execution::ExecutionEngine, Codegen},
    runtime::ValueRef,
};

pub fn run() {
    Codegen::install_execution_targets();

    let mut rl = DefaultEditor::new().expect("cannot create a repl");

    let global_environment = Box::leak(Box::new(Default::default()));

    loop {
        let readline = rl.readline("> ");

        match readline {
            Ok(line) => {
                rl.add_history_entry(line.as_str())
                    .expect("cannot add to the history");

                let mut codegen = Codegen::new(global_environment)
                    .install_error_handling()
                    .install_primitives()
                    .install_global_environment();

                let engine = ExecutionEngine::try_new(codegen.module)
                    .unwrap()
                    .install_primitive_symbols(&codegen.environment)
                    .install_global_environment(&codegen);

                match eval_line(&mut codegen, &engine, line) {
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

fn eval_line(
    codegen: &mut Codegen,
    engine: &ExecutionEngine,
    line: String,
) -> Result<ValueRef, String> {
    let value = crate::Parser::new()
        .parse(&line)
        .map_err(|err| err.to_string())?;

    let term = value.specialize().map_err(|err| {
        format!(
            "[error] Could not specialize expression on code [{}:{}:{}]: {}",
            err.r_source_file, err.r_source_line, err.r_source_column, err.message
        )
    })?;

    let converted = term.convert();

    codegen
        .compile_main(converted)
        .map_err(|err| format!("[error] Could not compile expression: {err:?}"))?;

    codegen.dump_module();
    codegen.verify_module().unwrap_or_else(|error| {
        for line in error.split("\n") {
            println!("[error*] {}", line);
        }

        panic!("Module verification failed")
    });

    let f: extern "C" fn() -> ValueRef =
        unsafe { std::mem::transmute(engine.get_function_address("main")) };

    Ok(f())
}
