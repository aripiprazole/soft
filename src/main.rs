

pub mod repl;

use clap::Parser;
use miette::IntoDiagnostic;

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

fn main() -> miette::Result<()> {

    // Install the panic handler.
    bupropion::install(bupropion::BupropionHandlerOpts::new).into_diagnostic()?;

    // Parse the command line arguments.
    let args = Args::parse();

    if args.repl {
        repl::repl();
    }

    Ok(())
}
