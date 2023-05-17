use soft_compiler::parser::parse;

fn main() {
    let code = "(ata\n be\n :de f (a b cs) 1234) (a b c)";

    let parsed = parse(code).unwrap();

    for expr in parsed {
        println!("{}", expr);
    }
}
