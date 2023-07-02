use rustyline::DefaultEditor;
use soft::{environment::Environment, value::Value};

fn main() {
    let args = std::env::args().skip(1).collect::<Vec<_>>();

    if args.len() == 1 {
        let file = args[0].clone();
        run_file(file);
    } else if args.is_empty() {
        run_repl();
    } else {
        eprintln!("Usage: soft <file>");
        return;
    }
}

fn run_repl() {
    let mut rl = DefaultEditor::new().unwrap();

    let mut env = Environment::new(None);
    env.register_intrinsics();

    loop {
        let readline = rl.readline("> ");
        match readline {
            Ok(line) => {
                rl.add_history_entry(line.clone()).unwrap();
                if let Some(res) = execute_code(line, "<repl>".to_string(), &mut env) {
                    println!("{}", res)
                }
            }
            Err(_) => break,
        }
    }
}

fn run_file(file: String) {
    let Ok(code) = std::fs::read_to_string(&file) else {
        eprintln!("error: could not read file '{}'", &file);
        return;
    };

    let mut env = Environment::new(Some(file.to_string()));
    env.register_intrinsics();

    execute_code(code, file, &mut env);
}

fn execute_code(code: String, file: String, env: &mut Environment) -> Option<Value> {
    let mut result = None;

    for value in soft::reader::read(&code, Some(file)).unwrap() {
        let evaluated = value.run(env);
        match evaluated {
            Ok(res) => {
                result = Some(res);
            }
            Err(err) => {
                print_error(err, env);
                return None;
            }
        }
    }

    result
}

fn print_error(err: soft::error::RuntimeError, env: &mut Environment) {
    println!("error: {} at {}", err, env.location.clone());
    let unwind = env.unwind();

    for frame in unwind.iter() {
        println!(
            "  in {} at {}",
            frame.name.clone().unwrap_or("unknown".to_string()),
            frame.location
        );
    }
}
