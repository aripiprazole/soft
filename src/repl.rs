use std::path::PathBuf;

use rustyline::error::ReadlineError;
use rustyline::validate::MatchingBracketValidator;
use rustyline::Editor;
use rustyline::{Completer, Helper, Highlighter, Hinter, Validator};

fn get_history_path() -> Option<PathBuf> {
    let home_env = std::env::var("HOME").ok()?;
    let path = format!("{home_env}/.soft.history");
    Some(PathBuf::from(path))
}

#[derive(Completer, Helper, Highlighter, Hinter, Validator)]
struct InputValidator {
    #[rustyline(Validator)]
    brackets: MatchingBracketValidator,
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
        let readline = rl.readline("> ");

        match readline {
            Ok(line) => {
                rl.add_history_entry(line.as_str()).unwrap();
                println!("Line: {}", line);
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
