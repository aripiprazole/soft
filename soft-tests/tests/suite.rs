#![feature(custom_test_frameworks)]
#![test_runner(soft_tests::test_runner)]

use soft::{parse, run, Environment};
use soft_tests::mk_test;
use std::fmt::Write;

mk_test! { "/../soft-suite", |code, file_name| {
    let mut result = String::new();

    let mut env = Environment::new(None);
    env.register_intrinsics();

    for expr in parse(&code, Some(file_name.into())).unwrap() {
        match run(&mut env,expr) {
            Ok(res) => writeln!(&mut result, "ok: {}", res).unwrap(),
            Err(err) => {
                writeln!(&mut result, "error: {err} at {}", env.find_first_location()).unwrap()
            }
        }
    }
    result
} }
