use super::config::GitConfig;
use auth_git2::GitAuthenticator;
use git2::{Branch, BranchType, Error, Oid, Repository, Signature};
use serde::{de::DeserializeOwned, Serialize};
use std::process::Command;

pub struct Git {
    repository: Repository,
    signature: Signature<'static>,
    pub config: GitConfig,
    auth: GitAuthenticator,
}

#[derive(Clone)]
pub struct EnhancedCommit<N> {
    pub id: Oid,
    pub title: String,
    pub description: Option<String>,
    pub note: Option<N>,
}

#[allow(dead_code)]
enum PushMode {
    Normal,
    Force,
    ForceWithLease,
}

impl Git {
    /// Open a repository at the given path
    /// Also load the signature from the .gitconfig
    pub fn open(path: &str) -> Self {
        let current_dir = std::env::current_dir().expect("cannot open current directory");
        let path = current_dir.join(path);
        let repository = Repository::discover(path).expect("repository not found");
        let gitconfig = GitConfig::open().expect("git config not found");

        let signature = Signature::now(&gitconfig.user.name, &gitconfig.user.email)
            .expect("cannot compute signature");

        Git {
            repository,
            signature,
            config: gitconfig,
            auth: GitAuthenticator::new(),
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

            let Some(commit) = self.find_commit(oid) else {
                continue;
            };
            commits.push(commit);
        }
        commits.reverse();
        commits
    }

    fn push(&self, origin: &str, branch: &str, mode: PushMode) {
        println!("pushing {}:{}", origin, branch);
        let fetch_refname = format!("refs/heads/{}", branch);
        let git_config = self.repository.config().unwrap();
        let mut push_options = git2::PushOptions::new();

        let mut remote_callbacks = git2::RemoteCallbacks::new();
        remote_callbacks.credentials(self.auth.credentials(&git_config));

        remote_callbacks.push_negotiation(|remote_updates| {
            let null = git2::Oid::zero();
            for remote_update in remote_updates {
                // It's a new branch
                if remote_update.src() == null {
                    println!("{}:{} is a new branch", origin, branch);
                    return Ok(());
                }
                // No need to push
                if remote_update.src() == remote_update.dst() {
                    println!("{}:{} is up to date", origin, branch);
                    return Err(git2::Error::from_str("no need to push"));
                }
                return match mode {
                    PushMode::Normal => {
                        // last commit of remote has to be known in current branch
                        Err(Error::from_str("not yet implemented"))
                    }
                    PushMode::Force => Ok(()),
                    PushMode::ForceWithLease => {
                        // Comparing src with local origin
                        let remote_origin_oid = remote_update.src();
                        // Get the head of this branch
                        let local_origin_oid = {
                            let local_origin_name = remote_update.src_refname().unwrap();
                            let upstream_name =
                                local_origin_name.strip_prefix("refs/heads/").unwrap();
                            self.repository
                                .find_reference(&format!(
                                    "refs/remotes/{}/{}",
                                    origin, upstream_name
                                ))
                                .ok()
                                .and_then(|reference| reference.peel_to_commit().ok())
                                .map(|commit| commit.id())
                                .unwrap()
                        };
                        if remote_origin_oid == local_origin_oid {
                            Ok(())
                        } else {
                            println!("{}:{} have diverged, not pushing", origin, branch);
                            Err(Error::from_str("Origins have divered"))
                        }
                    }
                };
            }
            println!("There were no negotiation");
            Err(git2::Error::from_str("No negotiation"))
        });

        push_options.remote_callbacks(remote_callbacks);

        let mut remote = self
            .repository
            .find_remote(origin)
            .expect("Cannot find origin");
        let result = remote.push(
            &[format!("+{}", fetch_refname).as_str()],
            Some(&mut push_options),
        );
        if result.is_ok() {
            println!("{}:{} pushed", origin, branch);
        }
        return;
    }

    pub fn push_force_with_lease(&self, origin: &str, branch: &str) {
        self.push(origin, branch, PushMode::ForceWithLease)
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
        let Ok(note) = serde_json::to_string(&note) else {
            return Err(());
        };

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
        let commit = self
            .repository
            .find_commit(oid)
            .map_err(|err| println!("{:?}", err))?;

        self.repository
            .branch(branch, &commit, true)
            .map_err(|err| println!("{:?}", err))?;

        Ok(())
    }

    /// Open the given file with the user's editor and returns the content of this file
    pub fn edit_file(&self, file_path: &str) -> Result<String, ()> {
        let output = Command::new(&self.config.core.editor)
            .arg(file_path)
            .status()
            .expect("Failed to execute command");
        let true = output.success() else {
            return Err(());
        };
        let content = std::fs::read_to_string(file_path).unwrap();
        Ok(content)
    }
}
