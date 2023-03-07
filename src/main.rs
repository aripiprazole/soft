use clap::Parser;
use soft::cli::Options;

fn main() {
    let options = Options::parse();

    soft::repl::run(&options);
}
