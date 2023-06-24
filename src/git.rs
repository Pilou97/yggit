use git2::{Repository, Signature};
use serde::Deserialize;
use std::{fs::File, io::Read, path::Path};

pub struct Git {
    pub repository: Repository,
    pub signature: Signature<'static>,
}

#[derive(Deserialize, Debug)]
struct GitConfig {
    user: UserConfig,
}

#[derive(Deserialize, Debug)]
struct UserConfig {
    email: String,
    name: String,
}

impl Git {
    /// Try to find a repository in the given path
    /// Otherwise, it tries to open the parent directory
    fn find_repository(path: &Path) -> Repository {
        let repository = Repository::open(path);
        match repository {
            Ok(repository) => repository,
            Err(_) => {
                let path = path.parent().expect("repository not found");
                Self::find_repository(path)
            }
        }
    }

    fn find_gitconfig(path: &Path) -> GitConfig {
        let file = path.join(".gitconfig");
        let file = File::open(file);
        match file {
            Ok(mut file) => {
                let mut contents = String::new();
                file.read_to_string(&mut contents)
                    .expect("Failed to read the file.");
                let git_config: GitConfig =
                    toml::from_str(&contents).expect("Git config parsing error");
                git_config
            }
            Err(_) => {
                let path = path.parent().expect(".gitconfig not found");
                Self::find_gitconfig(path)
            }
        }
    }

    /// Open a repository at the given path
    /// Also load the signature from the .gitconfig
    pub fn open(path: &str) -> Self {
        let current_dir = std::env::current_dir().expect("cannot open current directory");
        let path = current_dir.join(path);
        let repository = Self::find_repository(path.as_path());
        let gitconfig = Self::find_gitconfig(path.as_path());

        let signature = Signature::now(&gitconfig.user.name, &gitconfig.user.email)
            .expect("cannot compute signature");

        Git {
            repository,
            signature,
        }
    }
}
