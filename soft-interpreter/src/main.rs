use soft::{parse, run, Environment};

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
        if let Err(err) = run(&mut env, expr) {
            eprintln!("error: {err}");
            eprintln!("  at {}", env.find_first_location());
            env.unwind();
        }
    }
}
