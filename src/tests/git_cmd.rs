use crate::git::config::{Core, GitConfig, User, Yggit};
use git2::Oid;
use serde::Serialize;
use std::{
    io::Write,
    process::{Command, Stdio},
};
use tempfile::TempDir;

macro_rules! execute_commands {
    ($($cmd:expr $(, $arg:expr)*)* ) => {
        {
            $(
                let cmd_string = format!("{} {}", $cmd, vec![$($arg),*].join(" "));
                println!("{}", cmd_string);
                let child = Command::new($cmd)
                    $(.arg($arg))*
                    .stdout(Stdio::piped())
                    .spawn()
                    .expect("Failed to spawn child process");

                let output = child.wait_with_output().expect("Failed to read stdout");
                if !output.status.success() {
                    panic!("the command did not succeed");
                }
                String::from_utf8(output.stdout).expect("should be parasable")
            )*
        }
    };
}

macro_rules! git {
    ($self:ident, $($args:expr),* ) => {
        execute_commands!("git", "-C", &$self.path(), $($args),*)
    };
}

macro_rules! git_config {
    ($self:ident, $($args:expr),* ) => {
        git!($self, "config", "--local", $($args),*)
    };
}

pub struct GitCmd {
    bare: Option<TempDir>,
    directory: TempDir,
}

impl Clone for GitCmd {
    fn clone(&self) -> Self {
        let clone = TempDir::new().expect("directory should be created");
        let Some(ref bare) = self.bare else {
            todo!("no bare repository: impossible to clone")
        };

        execute_commands!(
            "git",
            "clone",
            &format!("file://{}", bare.path().to_str().unwrap().to_string()),
            &clone.path().to_str().unwrap().to_string()
        );

        let git = Self {
            bare: None,
            directory: clone,
        };

        git.init_config();

        git
    }
}

/// Helper that execute git command
///
/// So that git.rs can be tested against the git binary
impl GitCmd {
    /// Create a repository with a bare one
    pub fn init_bare(initial_branch: &str) -> Self {
        let bare = tempfile::Builder::new()
            .suffix(".git")
            .tempdir()
            .expect("git bare folder to be created");

        execute_commands!(
            "git",
            "-C",
            &bare.path().to_str().unwrap().to_string(),
            "init",
            "--initial-branch",
            initial_branch,
            "--bare"
        );

        // Then we clone it
        let clone = TempDir::new().expect("Directory should be created");

        execute_commands!(
            "git",
            "clone",
            &format!("file://{}", bare.path().to_str().unwrap().to_string()),
            &clone.path().to_str().unwrap().to_string()
        );

        let git = Self {
            bare: Some(bare),
            directory: clone,
        };

        git.init_config();

        git
    }

    /// This function has to be called in each constructor
    /// Later we can add an optional argument Config
    pub fn init_config(&self) {
        // TODO: put this in config.rs as dummy in test module
        let config = GitConfig {
            user: User {
                email: "example@example.com".to_string(),
                name: "Obi-wan".to_string(),
            },
            core: Core {
                editor: "theforce".to_string(), // The editor is not tested
            },
            yggit: Yggit {
                default_upstream: "origin".to_string(),
            },
        };

        git_config!(self, "user.email", config.user.email.as_str());
        git_config!(self, "user.name", config.user.name.as_str());
        git_config!(self, "core.editor", config.core.editor.as_str());
        git_config!(
            self,
            "yggit.defaultUpstream",
            config.yggit.default_upstream.as_str()
        );
        git_config!(self, "notes.rewriteRef", "refs/notes/commits");
    }

    /// Add a file to the repository
    pub fn new_file(&self, file_name: &str, content: &str) {
        let path = self.directory.path().join(file_name);
        let mut file = std::fs::File::create(path).expect("file should be created");
        file.write_all(content.as_bytes())
            .expect("should have written file to disk");
    }

    /// Add all files to the next commit
    pub fn add_all(&self) {
        let _ = git!(self, "add", ".");
    }

    /// Commit the change
    pub fn commit(&self, commit_name: &str) -> Oid {
        let _ = git!(self, "commit", "-m", commit_name);
        let oid = git!(self, "rev-parse", "HEAD");
        let oid = oid.trim();

        Oid::from_str(&oid).unwrap()
    }

    pub fn add_note<N>(&self, oid: Oid, note: &N)
    where
        N: Serialize,
    {
        let json = serde_json::to_string(note).expect("note");
        git!(self, "notes", "add", "-m", &json, &oid.to_string());
    }

    pub fn push(&self) {
        git!(self, "push", "--force");
    }

    /// Returns the path of the repository
    pub fn path(&self) -> String {
        self.directory.path().to_str().unwrap().to_string()
    }

    /// Modifies the title of HEAD
    pub fn amend(&self, title: &str) {
        git!(self, "commit", "--amend", "-m", title);
    }

    /// pull the repository
    pub fn pull(&self) {
        git!(self, "pull");
    }

    pub fn create_branch(&self, branch_name: &str) {
        git!(self, "checkout", "-b", branch_name);
    }

    /// Returns the commit id of the branch
    pub fn get_commit_of_branch(&self, branch_name: &str) -> Oid {
        let result = git!(self, "rev-parse", branch_name);
        println!("oid: {}", result);
        Oid::from_str(result.trim()).unwrap()
    }
}
