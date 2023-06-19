use soft_interpreter::Eval;
use soft_interpreter::{intrinsics, parse, Environment};

fn main() {
    let mut environment = Environment::new(None);

    environment.extend("call", intrinsics::call);

    for expr in parse("(a)").unwrap() {
        println!("{:?}", expr.eval(&mut environment));
    }
}
