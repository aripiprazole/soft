use soft::environment::Environment;

fn main() {
    let code = "
        (set* fib (fn* (n)
            (if (< n 2)
                n
                (+ (fib (- n 1)) (fib (- n 2))))))
        
        (fib 10)
    ";

    let mut env = Environment::new(None);
    env.register_intrinsics();

    for value in soft::reader::read(code, None).unwrap() {
        let evaluated = value.run(&mut env);
        match evaluated {
            Ok(value) => println!("ok: {}", value),
            Err(err) => println!("error: {}", err),
        }
    }
}
