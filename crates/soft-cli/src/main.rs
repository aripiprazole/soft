fn main() {
    let ata = "(:ata 1 (lambda (a b c) 2))";

    let parsed = soft_compiler::parser::parse(ata).unwrap();

    for expr in parsed {
        println!("{:#?}", soft_compiler::specialize::specialize(&expr));
    }
}
