use super::config::GitConfig;
use git2::{
    Branch, BranchType, Cred, CredentialType, Error, FetchOptions, Oid, PushOptions,
    RebaseOperationType, RemoteCallbacks, Repository, Signature,
};
use serde::{de::DeserializeOwned, Serialize};
use std::{path::Path, process::Command};

pub struct Git {
    pub repository: Repository,
    pub signature: Signature<'static>,
    config: GitConfig,
}

#[derive(Clone)]
pub struct EnhancedCommit<N> {
    pub id: Oid,
    pub title: String,
    pub description: Option<String>,
    pub note: Option<N>,
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

    /// Open a repository at the given path
    /// Also load the signature from the .gitconfig
    pub fn open(path: &str) -> Self {
        let current_dir = std::env::current_dir().expect("cannot open current directory");
        let path = current_dir.join(path);
        let repository = Self::find_repository(path.as_path());
        let gitconfig = GitConfig::from_directory(path.as_path()).expect("git config not found");

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
    pub fn main_branch(&self) -> Option<Branch> {
        let branches = ["main", "master"];

        for branch in branches {
            let branch = self.repository.find_branch(branch, BranchType::Local);
            if let Ok(branch) = branch {
                return Some(branch);
            }
        }
        None
    }

    /// List the commit in a repository and the attached note
    pub fn list_commits<N>(&self) -> Vec<EnhancedCommit<N>>
    where
        N: DeserializeOwned,
    {
        // Find the commit of the "main" branch
        let main_branch = self.main_branch().expect("main/master to exist");

        let main_commit = main_branch.get().peel_to_commit().unwrap();

        let mut revwalk = self.repository.revwalk().unwrap();
        revwalk.push_head().unwrap();

        let mut commits = Vec::default();

        for oid in revwalk {
            let oid = oid.unwrap();

            if oid == main_commit.id() {
                break;
            }

            let Some(commit) = self.find_commit(oid) else {continue;};
            commits.push(commit);
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

    /// Returns the local id of the head of origin/{branch}
    pub fn find_local_remote_head(&self, branch: &str) -> Option<Oid> {
        let Self { repository, .. } = self;
        // Get the reference of the branch
        let reference = format!("refs/remotes/origin/{}", branch);

        // Get the head of this branch
        repository
            .find_reference(&reference)
            .ok()
            .and_then(|reference| reference.peel_to_commit().ok())
            .map(|commit| commit.id())
    }

    /// Returns the remote head of origin/{branch}
    ///
    /// It will fetch the repository
    /// Get the head
    /// Revert the fetch
    pub fn find_remote_head(&self, branch: &str) -> Option<Oid> {
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
            (None, None, None) => None,
            (None, None, Some(_)) => {
                println!("remote and reference should exists possible");
                None
            }
            (None, Some(_), None) => {
                println!("odd");
                None
            }
            (None, Some(remote_commit), Some(_)) => {
                println!("No local commits, but remote one");
                Some(remote_commit.id())
            }
            (Some(_), None, None) => None,
            (Some(local_commit), None, Some(mut reference)) => {
                reference
                    .set_target(local_commit.id(), "revert fetch")
                    .expect("revert fetch error");
                println!("reference and remote should exists");
                None
            }
            (Some(_), Some(remote_commit), None) => {
                println!("local commit exists, remote too, but no references...");
                Some(remote_commit.id())
            }
            (Some(local_commit), Some(remote_commit), Some(mut reference)) => {
                reference
                    .set_target(local_commit.id(), "revert fetch")
                    .expect("revert fetch error");
                Some(remote_commit.id())
            }
        }
    }

    ///  Returns the commit to head of branch and head of branch/origin
    pub fn head_of(&self, branch: &str) -> Option<Oid> {
        let local_reference_name = format!("refs/heads/{}", branch);

        // Get the local commit
        self.repository
            .find_reference(&local_reference_name)
            .ok()
            .and_then(|reference| reference.peel_to_commit().ok())
            .map(|commit| commit.id())
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

    /// Set the note of a given oid
    ///
    /// The note will be serialize to json format
    pub fn set_note<N>(&self, oid: Oid, note: N) -> Result<(), ()>
    where
        N: Serialize,
    {
        let Ok(note) = serde_json::to_string(&note) else {return Err(())};

        self.repository
            .note(&self.signature, &self.signature, None, oid, &note, true)
            .map(|_| ())
            .map_err(|_| ())
    }

    /// Retrieve a commit with its node
    pub fn find_commit<N>(&self, oid: Oid) -> Option<EnhancedCommit<N>>
    where
        N: DeserializeOwned,
    {
        // Get the commit
        let commit = self.repository.find_commit(oid).ok()?;
        // Get the associated note
        let note: Option<N> = self
            .repository
            .find_note(None, oid)
            .map(|note| note.message().map(|str| str.to_string()))
            .ok()
            .flatten()
            .and_then(|string| {
                // Take the last line
                // So that it's compatible with merging fixup commits
                // When two commits are merged, the note are also merged
                // The note of the most recent commit is taking into account then
                string.split('\n').last().map(ToString::to_string)
            })
            .and_then(|str| serde_json::from_str(&str).ok());

        // Get the title and the description
        let mut message = commit.message().unwrap_or_default().splitn(2, '\n');
        // Title is on the first line of the message
        let title = message.next().unwrap_or_default().to_string();
        // Remaining lines are for the description
        let description = message.next().map(str::to_string);

        Some(EnhancedCommit {
            id: oid,
            title,
            description,
            note,
        })
    }

    /// Set the head of the given branch to the given commit
    pub fn set_branch_to_commit(&self, branch: &str, oid: Oid) -> Result<(), ()> {
        let Ok(commit) = self.repository.find_commit(oid) else {
            println!("commit does not exist");
            return Err(())
        };

        self.repository
            .branch(branch, &commit, true)
            .map(|_| ())
            .map_err(|err| {
                println!("{:?}", err);
                ()
            })
    }

    /// Open the given file with the user's editor and returns the content of this file
    pub fn edit_file(&self, file_path: &str) -> Result<String, ()> {
        let output = Command::new(&self.config.core.editor)
            .arg(file_path)
            .status()
            .expect("Failed to execute command");
        let true = output.success() else {return Err(());};
        let content = std::fs::read_to_string(file_path).unwrap();
        Ok(content)
    }

    /// Open or continue a rebase
    pub fn rebase<F>(&self, onto: Branch, fct: F) -> Result<(), ()>
    where
        F: Fn(Oid, &Self) -> Result<(), ()>,
    {
        let branch = self
            .repository
            .reference_to_annotated_commit(onto.get())
            .map_err(|_| ())?;

        let mut rebase = match self.repository.open_rebase(None) {
            Ok(rebase) => rebase,
            Err(_) => self
                .repository
                .rebase(None, None, Some(&branch), None)
                .expect("Failed to start rebase"),
        };

        while let Some(operation) = rebase.next() {
            match operation {
                Ok(operation) => match operation.kind() {
                    Some(RebaseOperationType::Pick) => {
                        let commit_id = operation.id();
                        rebase
                            .commit(None, &self.signature, None)
                            .expect("Failed to commit during rebase");
                        let res = fct(commit_id, &self);
                        match res {
                            Ok(()) => {}
                            Err(_) => return Ok(()),
                        }
                    }
                    _ => continue,
                },
                _ => continue,
            }
        }
        rebase.finish(Some(&self.signature)).map_err(|_| ())
    }
}
