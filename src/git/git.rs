use super::config::GitConfig;
use git2::{BranchType, Oid, RebaseOperation, RebaseOptions, Repository, Signature};
use serde::{de::DeserializeOwned, Serialize};
use std::fs;
use std::path::Path;
use std::process::Command;

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

    /// List the commit in a repository and the attached note
    pub fn list_commits<N>(&self, branch: &str) -> Vec<EnhancedCommit<N>>
    where
        N: DeserializeOwned,
    {
        let branch = self
            .repository
            .find_branch(branch, BranchType::Local)
            .unwrap();

        let branch_head = branch.get().peel_to_commit().unwrap();

        let mut revwalk = self.repository.revwalk().unwrap();
        revwalk.push_head().unwrap();

        let mut commits = Vec::default();

        for oid in revwalk {
            let oid = oid.unwrap();

            if oid == branch_head.id() {
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

    pub(crate) fn start_rebase(&self, branch: &str) {
        let onto = self
            .repository
            .find_branch(branch, BranchType::Local)
            .expect("the branch should exist");

        let branch = self
            .repository
            .reference_to_annotated_commit(onto.get())
            .expect("branch should exist");

        let mut options = RebaseOptions::default();
        let options = options.rewrite_notes_ref("NULL"); // hm, not sure...

        let _ = self
            .repository
            .rebase(None, None, Some(&branch), Some(options))
            .expect("rebase should have started");
    }

    pub(crate) fn write_todo(&self, instructions: &str) {
        let path = ".git/rebase-merge/git-rebase-todo";
        fs::write(path, instructions).expect("todo should be written");
    }

    pub(crate) fn rebase_continue(&self) {
        Command::new("git")
            .arg("rebase")
            .arg("--continue")
            .spawn()
            .expect("should work");
    }
}
