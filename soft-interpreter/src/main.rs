use soft::environment::Environment;

fn main() {
    let mut env = Environment::new(None);
    env.register_intrinsics();

    let args = std::env::args().skip(1).collect::<Vec<_>>();

    if args.len() != 1 {
        eprintln!("Usage: soft <file>");
        return;
    }

    let file = args[0].clone();

    let Ok(code) = std::fs::read_to_string(&file) else {
        eprintln!("Error: could not read file '{}'", &file);
        return;
    };

    let mut env = Environment::new(Some(file.to_string()));
    env.register_intrinsics();

    for value in soft::reader::read(&code, Some(file)).unwrap() {
        let evaluated = value.run(&mut env);
        match evaluated {
            Ok(_) => (),
            Err(err) => {
                println!("error: {} at {}", err, env.location.clone());
                let unwind = env.unwind();

                for frame in unwind.iter() {
                    println!(
                        "  in {} at {}",
                        frame.name.clone().unwrap_or("unknown".to_string()),
                        frame.location
                    );
                }

                break;
            }
        }
    }
}
