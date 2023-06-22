#![feature(custom_test_frameworks)]
#![test_runner(soft_tests::test_runner)]

use soft::{environment::Environment, reader::read};
use soft_tests::mk_test;
use std::fmt::Write;

mk_test! { "/../soft-suite", |code, file_name| {
    let mut result = String::new();

    let mut env = Environment::new(None);
    env.register_intrinsics();

    for expr in read(&code, Some(file_name)).unwrap() {
        match expr.run(&mut env) {
            Ok(res) => writeln!(&mut result, "ok: {}", res).unwrap(),
            Err(err) => {
                writeln!(&mut result, "error: {err} at {}", env.last_frame().located_at).unwrap()
            }
        }
    }
    result
} }
