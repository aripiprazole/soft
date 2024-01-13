use std::path::PathBuf;

use clap::Parser;
use miette::IntoDiagnostic;
use rustyline::{
    error::ReadlineError, validate::MatchingBracketValidator, Completer, Editor, Helper,
    Highlighter, Hinter, Validator,
};
use soft::{eval::Environment, Expr, Term};

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Loads a file to use as input.
    #[arg(short, long)]
    load: Option<String>,

    #[arg(short = 'X', long)]
    exe: Option<String>,

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
    let environment = Environment::default();
    if let Some(expr) = args.exe {
        exec(expr, &environment);
    }
    if args.repl {
        repl(&environment);
    }

    Ok(())
}

fn get_history_path() -> Option<PathBuf> {
    let home_env = std::env::var("HOME").ok()?;
    let path = format!("{home_env}/.soft.history");
    Some(PathBuf::from(path))
}

pub fn exec(content: String, environment: &Environment) {
    let value = soft::parser::parse_sexpr(&content)
        .and_then(|sexpr| Expr::try_from(sexpr).map_err(|error| error.into()))
        .and_then(|expr| expr.expand(environment))
        .and_then(|expr| expr.eval(environment).eval_into_result());

    match value {
        Ok(value) => println!("{}", value.readback()),
        Err(error) => eprintln!("{}", Term::from(error)),
    }
}

pub fn repl(environment: &Environment) {
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
                exec(line, environment)
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
