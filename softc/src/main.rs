//
use clap::Parser;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Loads a file to use as input.
    #[arg(short, long)]
    load: Option<String>,

    /// Starts a repl session.
    #[arg(short, long)]
    repl: bool
}

fn main() {
    let args = Args::parse();

    
}