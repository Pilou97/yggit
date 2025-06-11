use clap::{arg, command, Args, Parser, Subcommand};
use git2::Repository;
use yggit_core::push;
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

    let repository = Repository::discover(".").expect("you need to open a valid repository");
    let config = repository.config().expect("config cannot be loaded found");
    let name = config
        .get_string("user.name")
        .expect("you need to set a name in your git config");
    let email = config
        .get_string("user.email")
        .expect("you need to set an email in your git config");
    let editor = config
        .get_string("core.editor")
        .expect("you need to define editor");

    let git = GitClient::new(&repository);
    let db = GitDatabase::new(&repository, name, email);
    let editor = GitEditor::new(editor.to_string());

    match args.command {
        Commands::Push(Push { force, onto }) => match push(git, db, editor, force, onto) {
            Ok(()) => println!("everything is fine"),
            Err(_) => println!("there was an error"),
        },
    }
}
