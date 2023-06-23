use git2::{Oid, Repository, Signature};
use serde::{Deserialize, Serialize};
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

#[derive(Clone)]
pub struct EnhancedCommit {
    pub id: Oid,
    pub message: String,
    pub note: Option<Note>,
}

#[derive(Clone, Serialize, Deserialize)]
pub enum Note {
    Target { branch: String },
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

    /// List the commit in a repository and the attached note
    pub fn list_commits(&self) -> Vec<EnhancedCommit> {
        let main = "main";
        // Find the commit of the "main" branch
        let main_branch = self
            .repository
            .find_branch(main, git2::BranchType::Local)
            .unwrap();
        let main_commit = main_branch.get().peel_to_commit().unwrap();

        let mut revwalk = self.repository.revwalk().unwrap();
        revwalk.push_head().unwrap();

        let mut commits = Vec::default();

        for oid in revwalk {
            let oid = oid.unwrap();

            if oid == main_commit.id() {
                break;
            }

            let commit = self.repository.find_commit(oid).unwrap();

            let note: Option<Note> = self
                .repository
                .find_note(None, oid)
                .map(|note| note.message().map(|str| str.to_string()))
                .ok()
                .flatten()
                .and_then(|string| {
                    // Take the last line
                    // So that it's compatible with fixup commits
                    string.split('\n').last().map(ToString::to_string)
                })
                .and_then(|str| serde_json::from_str(&str).ok());

            commits.push(EnhancedCommit {
                id: oid,
                message: commit.message().unwrap().to_string(),
                note,
            });
        }
        commits.reverse();
        commits
    }

    /// Push force a branch
    pub fn push_force(&self, branch: &str) {
        let fetch_refname = format!("refs/heads/{}", branch);
        let mut remote = self
            .repository
            .find_remote("origin")
            .expect("Cannot find origin");

        remote
            .connect(git2::Direction::Push)
            .expect("Cannot connect to remote in Push direction");

        // The + character means that the branch is forced pushed
        remote
            .push(&[format!("+{}", fetch_refname)], None)
            .expect("Push force failed");
    }
}
