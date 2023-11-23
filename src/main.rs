use std::path::PathBuf;

use clap::Parser;
use miette::IntoDiagnostic;
use rustyline::{
    error::ReadlineError, validate::MatchingBracketValidator, Completer, Editor, Helper,
    Highlighter, Hinter, Validator,
};

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Loads a file to use as input.
    #[arg(short, long)]
    load: Option<String>,

    /// Starts a repl session.
    #[arg(short, long)]
    repl: bool,
}

#[derive(Completer, Helper, Highlighter, Hinter, Validator)]
struct InputValidator {
    #[rustyline(Validator)]
    brackets: MatchingBracketValidator,
}

fn main() -> miette::Result<()> {
    // Install the panic handler.
    bupropion::install(bupropion::BupropionHandlerOpts::new).into_diagnostic()?;

    // Parse the command line arguments.
    let args = Args::parse();

    if args.repl {
        repl();
    }

    Ok(())
}

fn get_history_path() -> Option<PathBuf> {
    let home_env = std::env::var("HOME").ok()?;
    let path = format!("{home_env}/.soft.history");
    Some(PathBuf::from(path))
}

pub fn repl() {
    let mut rl = Editor::new().expect("cannot create repl");
    let path = get_history_path();
    let h = InputValidator {
        brackets: MatchingBracketValidator::new(),
    };

    rl.set_helper(Some(h));

    if let Some(path) = path.clone() {
        if rl.load_history(&path).is_err() {
            println!("No previous history.");
        }
    }

    loop {
        match rl.readline("> ") {
            Ok(line) => {
                rl.add_history_entry(line.as_str()).unwrap();
                println!("Line: {}", line)
            }
            Err(ReadlineError::Interrupted) => {
                println!("Interrupted");
                break;
            }
            Err(ReadlineError::Eof) => {
                break;
            }
            Err(err) => {
                println!("Error: {err:?}");
                break;
            }
        }
    }

    if let Some(path) = path {
        let _ = rl.append_history(&path);
    }
}
