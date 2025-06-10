use clap::{arg, command, Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "yggit")]
#[command(about = "Git stacked workflow manager", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Push(Push),
}

#[derive(Debug, Args)]
pub struct Push {
    /// use --force to force the branch updates
    /// by default it has the behavior of force-with-lease
    #[arg(short, long, default_value_t = false)]
    force: bool,
    #[arg(long)]
    /// The starting point of your branch
    onto: Option<String>,
}

fn main() {
    let args = Cli::parse();

    // todo: open the git config
    // todo: open the repository
    // todo: init the db
    // todo: init the editor

    match args.command {
        Commands::Push(Push { force, onto }) => {
            println!(
                "TODO: call yggit_core::push(git, db, editor, {}, {:?})",
                force, onto
            )
        }
    }
}
