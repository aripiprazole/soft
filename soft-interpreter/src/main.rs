use soft::Eval;
use soft::{intrinsics, parse, Environment};

fn main() {
    let mut environment = Environment::new(None);
    environment.extend("call", intrinsics::call);

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
        match expr.eval(&mut environment) {
            Ok(value) => println!("=> {}", value),
            Err(err) => {
                println!("{}", expr);
                eprintln!(
                    "Error: {}\n    at {}",
                    err,
                    environment.find_first_location()
                );
                environment.unwind();
            }
        }
    }
}
