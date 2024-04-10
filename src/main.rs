use clap::Parser;
use clap::Subcommand;
use commands::apply::Apply;
use commands::push::Push;
use commands::show::Show;
use git::Git;
use git::Terminal;

mod commands;
mod core;
mod git;
mod parser;
mod tests;

#[derive(Debug, Parser)] // requires `derive` feature
#[command(name = "yggit")]
#[command(version, about = "Git project manager", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Push(Push),
    Show(Show),
    Apply(Apply),
}

fn main() {
    let args = Cli::parse();

    let git = Git::<Terminal>::open(".").unwrap();

    match args.command {
        Commands::Push(push) => push.execute(git),
        Commands::Show(show) => show.execute(git),
        Commands::Apply(apply) => apply.execute(git),
    }
    .unwrap()
}
