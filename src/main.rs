use clap::Parser;
use clap::Subcommand;
use commands::push::Push;
use commands::show::Show;
use commands::test::Test;
use commands::Execute;

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
    Push(Push),
    Show(Show),
    Test(Test),
}

fn main() {
    let args = Cli::parse();

    let _ = match args.command {
        Commands::Push(push) => push.execute(),
        Commands::Show(show) => show.execute(),
        Commands::Test(test) => test.execute(),
    };
}
