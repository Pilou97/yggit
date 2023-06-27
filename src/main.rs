use clap::Parser;
use clap::Subcommand;
use commands::push;

mod commands;
mod core;
mod git;
mod parser;

#[derive(Debug, Parser)] // requires `derive` feature
#[command(name = "git")]
#[command(about = "A fictional versioning CLI", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Push,
}

fn main() {
    let args = Cli::parse();

    match args.command {
        Commands::Push => push(),
    }
}
