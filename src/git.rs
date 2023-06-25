use git2::{
    Branch, Cred, CredentialType, Error, FetchOptions, Oid, PushOptions, RemoteCallbacks,
    Repository, Signature,
};
use serde::{Deserialize, Serialize};
use std::{fs::File, io::Read, path::Path};

pub struct Git {
    pub repository: Repository,
    pub signature: Signature<'static>,
    config: GitConfig,
}

#[derive(Deserialize, Debug)]
struct GitConfig {
    user: UserConfig,
    yggit: Yggit,
}

#[derive(Deserialize, Debug)]
struct UserConfig {
    email: String,
    name: String,
}

#[derive(Deserialize, Debug)]
struct Yggit {
    #[serde(rename = "privateKey")]
    private_key: String,
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
            config: gitconfig,
        }
    }

    /// Returns the main branch of the repository
    ///
    /// The branch can be either main or master
    /// If main exists it will be returned as the main branch
    /// If main does not exist, master will be returned as the main branch
    pub fn main_branch(&self) -> Branch {
        let main = "main";
        let master = "master";

        let main_branch = self.repository.find_branch(main, git2::BranchType::Local);

        match main_branch {
            Ok(main) => main,
            Err(_) => self
                .repository
                .find_branch(master, git2::BranchType::Local)
                .expect("main branch not found"),
        }
    }

    /// List the commit in a repository and the attached note
    pub fn list_commits(&self) -> Vec<EnhancedCommit> {
        // Find the commit of the "main" branch
        let main_branch = self.main_branch();

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

    /// The callback to authenticate users
    ///
    /// For now, it only supports ssh
    fn auth_callback(
        &self,
    ) -> impl FnMut(&str, Option<&str>, CredentialType) -> Result<Cred, Error> {
        let private_key = self.config.yggit.private_key.clone();
        move |_, _, _| {
            let path = Path::new(&private_key);
            Cred::ssh_key("git", None, path, None)
        }
    }

    /// Returns the remote callback
    fn remote_callback(&self) -> RemoteCallbacks {
        let mut remote_callbacks = RemoteCallbacks::new();
        remote_callbacks.credentials(self.auth_callback());
        remote_callbacks
    }

    /// Check if the remote commit is the same as the local one
    /// Get the head of remote/{branch}
    /// Fetch
    /// Get the new head of remote/{branch}
    /// Check if the commits matched or not
    pub fn with_lease(&self, branch: &str) -> Result<(), ()> {
        let Self { repository, .. } = self;
        // Get the remote
        let mut remote = repository.find_remote("origin").expect("remote not found");
        // Get the reference of the branch
        let reference = format!("refs/remotes/origin/{}", branch);

        // Get the head of this branch
        let local_commit = repository
            .find_reference(&reference)
            .ok()
            .and_then(|reference| reference.peel_to_commit().ok());

        // Fetch the branch
        let mut options = FetchOptions::new();
        options.remote_callbacks(self.remote_callback());

        remote
            .fetch(&[branch], Some(&mut options), Some("fetch branch"))
            .expect("Fetching repository");

        // Get the new head
        let remote_commit = repository
            .find_reference(&reference)
            .ok()
            .and_then(|reference| reference.peel_to_commit().ok());

        // Get the reference object to the reference
        let reference = repository.find_reference(&reference).ok();

        // Change the reference to the old commit to revert the fetch

        match (local_commit, remote_commit, reference) {
            (None, None, None) => Ok(()),
            (None, None, Some(_)) => {
                println!("remote and reference should exists possible");
                Err(())
            }
            (None, Some(_), None) => {
                println!("odd");
                Err(())
            }
            (None, Some(_), Some(_)) => {
                println!("No local commits, but remote one");
                Err(())
            }
            (Some(_), None, None) => Ok(()),
            (Some(local_commit), None, Some(mut reference)) => {
                reference
                    .set_target(local_commit.id(), "revert fetch")
                    .expect("revert fetch error");

                println!("reference and remote should exists");
                Err(())
            }
            (Some(_), Some(_), None) => {
                println!("local commit exists, remote too, but no references...");
                Ok(())
            }
            (Some(local_commit), Some(remote_commit), Some(mut reference)) => {
                reference
                    .set_target(local_commit.id(), "revert fetch")
                    .expect("revert fetch error");
                if local_commit.id() != remote_commit.id() {
                    Err(())
                } else {
                    Ok(())
                }
            }
        }
    }

    /// Push force a branch
    pub fn push_force(&self, branch: &str) {
        let fetch_refname = format!("refs/heads/{}", branch);
        let mut remote = self
            .repository
            .find_remote("origin")
            .expect("Cannot find origin");

        let mut push_options = PushOptions::new();
        push_options.remote_callbacks(self.remote_callback());

        remote
            .connect_auth(git2::Direction::Push, Some(self.remote_callback()), None)
            .expect("Cannot connect to remote in Push direction");

        // The + character means that the branch is forced pushed
        remote
            .push(&[format!("+{}", fetch_refname)], Some(&mut push_options))
            .expect("Push force failed");
    }

    /// Delete a note
    pub fn delete_note(&self, oid: &Oid) {
        let _ = self
            .repository
            .note_delete(*oid, None, &self.signature, &self.signature);
    }
}
