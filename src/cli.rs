use clap::Parser;

#[derive(Parser, Debug, Default)]
#[command(author, version, about, long_about = None)]
pub struct Options {
    #[arg(short, long)]
    pub debug: bool,
}
