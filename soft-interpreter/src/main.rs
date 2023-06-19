use soft::{parse, Environment, RuntimeError, Value};
use soft::{Eval, Mode};

fn run(env: &mut Environment, mut expr: Value) -> Result<Value, RuntimeError> {
    env.mode = Mode::Macro;
    env.expanded = true;

    while env.expanded {
        env.expanded = false;
        expr = expr.eval(env)?;
    }

    println!("<= {}", expr);

    env.mode = Mode::Eval;
    expr.eval(env)
}

fn main() {
    let mut env = Environment::new(None);
    env.register_intrinsics();

    let args = std::env::args().skip(1).collect::<Vec<_>>();

    if args.len() != 1 {
        eprintln!("Usage: soft <file>");
        return;
    }

    let Ok(file) = std::fs::read_to_string(&args[0]) else {
        eprintln!("Error: could not read file '{}'", &args[0]);
        return;
    };

    for expr in parse(&file, Some(args[0].clone().into())).unwrap() {
        match run(&mut env, expr) {
            Ok(value) => println!("=> {}", value),
            Err(err) => {
                eprintln!("error: {err}");
                eprintln!("  at {}", env.find_first_location());
                env.unwind();
            }
        }
    }
}
