use clap::{arg, command, Args, Parser, Subcommand};
use git2::Repository;
use yggit_config::{Config, GitConfig};
use yggit_core::{push, show};
use yggit_db::GitDatabase;
use yggit_git::GitClient;
use yggit_ui::GitEditor;

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
    Show(Show),
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

#[derive(Debug, Args)]
pub struct Show {
    #[arg(long)]
    /// The starting point of your branch
    onto: Option<String>,
}

fn main() {
    let args = Cli::parse();

    // open the repository
    let repository = Repository::discover(".").expect("you need to open a valid repository");

    // init the dependencies
    let config = GitConfig::new(&repository).expect("invalid config");
    let git = GitClient::new(&repository);
    let db = GitDatabase::new(&repository, config.name().into(), config.email().into());
    let editor = GitEditor::new(config.editor().to_string());

    match args.command {
        Commands::Push(Push { force, onto }) => match push(git, db, editor, force, onto) {
            Ok(()) => println!("everything is fine"),
            Err(err) => println!("{}", err),
        },
        Commands::Show(Show { onto }) => match show(git, db, editor, onto) {
            Ok(()) => (),
            Err(err) => println!("{}", err),
        },
    }
}
