use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;

pub fn run() {
    let mut rl = DefaultEditor::new().expect("cannot create a repl");

    loop {
        let readline = rl.readline("> ");

        match readline {
            Ok(line) => {
                rl.add_history_entry(line.as_str())
                    .expect("cannot add to the history");

                let parsed = crate::Parser::new().parse(&line);

                if let Ok(term) = parsed {
                    let expr = crate::specialized::Term::specialize(term);
                    let converted = crate::conversion::convert(expr);
                    println!("{}", converted);
                } else {
                    println!("Cannot parse")
                }
            }
            Err(ReadlineError::Interrupted) => {
                println!("Bye bye...");
                break;
            }
            Err(ReadlineError::Eof) => {
                println!("CTRL-D");
                break;
            }
            Err(err) => {
                println!("Error: {:?}", err);
                break;
            }
        }
    }
}
