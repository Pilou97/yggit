use super::config::GitConfig;
use anyhow::{Context, Result};
use auth_git2::GitAuthenticator;
use git2::{Branch, BranchType, Error, ErrorCode, Oid, Repository, Signature};
use serde::{de::DeserializeOwned, Serialize};
use std::{
    path::PathBuf,
    process::Command,
    str::FromStr,
    sync::{Arc, Mutex},
};

pub struct Git {
    repository: Repository,
    signature: Signature<'static>,
    pub config: GitConfig,
    auth: GitAuthenticator,
}

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
    pub fn open(path: &str) -> Result<Self> {
        // The path can be absolute or not
        let path = if path.starts_with('/') {
            PathBuf::from_str(path).context("invalid absolute path")?
        } else {
            let current_dir = std::env::current_dir().context("cannot open current directory")?;
            current_dir.join(path)
        };
        let repository = Repository::discover(path).context("repository not found")?;
        let config = repository.config().context("config not found")?;
        let gitconfig = GitConfig::parse(config)?;
        let signature = Signature::now(&gitconfig.user.name, &gitconfig.user.email)
            .context("cannot compute signature")?;
        Ok(Git {
            repository,
            signature,
            config: gitconfig,
            auth: GitAuthenticator::new(),
        })
    }

    /// Delete a note
    ///
    /// Does not return any error when you delete nothing
    pub fn delete_note(&self, oid: &Oid) -> Result<()> {
        let result = self
            .repository
            .note_delete(*oid, None, &self.signature, &self.signature);
        if let Err(ref err) = result {
            if err.code() == ErrorCode::NotFound {
                return Ok(());
            }
        }
        result.context("cannot delete note")
    }

    /// Retrieve a commit with its node
    pub fn find_commit<N>(&self, oid: Oid) -> Option<EnhancedCommit<N>>
    where
        N: DeserializeOwned,
    {
        // Get the commit
        let commit = self.repository.find_commit(oid).ok()?;
        // Get the associated note
        let note: Option<N> = self.find_note(oid);
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
    pub fn set_branch_to_commit(&self, branch: &str, oid: Oid) -> Result<()> {
        let commit = self
            .repository
            .find_commit(oid)
            .context("Cannot find commit")?;

        self.repository
            .branch(branch, &commit, true)
            .context("Cannot find branch")?;

        Ok(())
    }

    /// Open the given file with the user's editor and returns the content of this file
    pub fn edit_file(&self, file_path: &str) -> Result<String> {
        let output = Command::new(&self.config.core.editor)
            .arg(file_path)
            .status()
            .context("Failed to open editor")?;
        let true = output.success() else {
            return Err(anyhow::Error::msg("Editor did not end successfully"));
        };
        let content =
            std::fs::read_to_string(file_path).context("Cannot read string from editor")?;
        Ok(content)
    }
}

#[cfg(test)]
mod tests {
    use git2::Oid;
    use serde::Serialize;
    use std::{
        io::Write,
        process::{Command, Stdio},
    };
    use tempfile::TempDir;

    use crate::git::config::{Core, GitConfig, User, Yggit};

    use super::Git;

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

    struct GitTmp {
        bare: Option<TempDir>,
        directory: TempDir,
    }

    impl Clone for GitTmp {
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
    impl GitTmp {
        /// Create a repository with a bare one
        fn init_bare(initial_branch: &str) -> Self {
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
        fn init_config(&self) {
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
        fn new_file(&self, file_name: &str, content: &str) {
            let path = self.directory.path().join(file_name);
            let mut file = std::fs::File::create(path).expect("file should be created");
            file.write_all(content.as_bytes())
                .expect("should have written file to disk");
        }

        /// Add all files to the next commit
        fn add_all(&self) {
            let _ = git!(self, "add", ".");
        }

        /// Commit the change
        fn commit(&self, commit_name: &str) -> Oid {
            let _ = git!(self, "commit", "-m", commit_name);
            let oid = git!(self, "rev-parse", "HEAD");
            let oid = oid.trim();

            Oid::from_str(&oid).unwrap()
        }

        fn add_note<N>(&self, oid: Oid, note: &N)
        where
            N: Serialize,
        {
            let json = serde_json::to_string(note).expect("note");
            git!(self, "notes", "add", "-m", &json, &oid.to_string());
        }

        fn push(&self) {
            git!(self, "push", "--force");
        }

        /// Returns the path of the repository
        fn path(&self) -> String {
            self.directory.path().to_str().unwrap().to_string()
        }

        /// Modifies the title of HEAD
        fn amend(&self, title: &str) {
            git!(self, "commit", "--amend", "-m", title);
        }

        /// pull the repository
        fn pull(&self) {
            git!(self, "pull");
        }

        fn create_branch(&self, branch_name: &str) {
            git!(self, "checkout", "-b", branch_name);
        }
    }

    #[test]
    fn test_open_repository() {
        let repo = GitTmp::init_bare("main");
        let _ = Git::open(&repo.path()).expect("repo should exist");
    }

    #[test]
    fn test_open_repository_not_found() {
        let tmp = TempDir::new().expect("the folder should be created");
        let result = Git::open(tmp.path().to_str().unwrap());
        assert!(result.is_err())
    }

    #[test]
    fn test_open_relative_repository() {
        let _ = Git::open(".");
    }

    /// helper that initialize a repository with one commit
    ///
    /// It returns the head and the repository
    fn init_repo_with_commit() -> (Oid, GitTmp) {
        let repo = GitTmp::init_bare("main");
        repo.new_file(
            "readme.md",
            concat!("# Star wars", "\n", "Hello there\n", "General Kenobi\n"),
        );
        repo.add_all();
        let oid = repo.commit("first commit");
        repo.add_note(oid, &"my super note".to_string());
        (oid, repo)
    }

    #[test]
    fn test_find_commit() {
        let (head, repo) = init_repo_with_commit();
        let git = Git::open(&repo.path()).expect("should be able to open the repository");
        let commit = git
            .find_commit::<String>(head)
            .expect("commit should be present");
        assert_eq!(commit.title, "first commit");
    }

    #[test]
    fn test_commit_not_found() {
        let (_, repo) = init_repo_with_commit();
        let git = Git::open(&repo.path()).expect("should be able to open the repository");
        let commit = git.find_commit::<String>(Oid::zero());
        assert!(commit.is_none())
    }

    #[test]
    fn test_get_note() {
        let (head, repo) = init_repo_with_commit();
        let git = Git::open(&repo.path()).expect("should be able to open the repository");
        let note = git
            .find_note::<String>(head)
            .expect("the note has to be present");
        assert_eq!(note, "my super note");
    }

    #[test]
    fn test_get_no_note() {
        let (_, repo) = init_repo_with_commit();
        let git = Git::open(&repo.path()).expect("should be able to open the repository");
        let note = git.find_note::<String>(Oid::zero());
        assert!(note.is_none());
    }

    #[test]
    fn test_delete_note() {
        let (head, repo) = init_repo_with_commit();
        let git = Git::open(&repo.path()).expect("should be able to open the repository");
        let note = git
            .find_note::<String>(head)
            .expect("the note has to be present");
        assert_eq!(note, "my super note");
        git.delete_note(&head).expect("not should be deleted");
        let note = git.find_note::<String>(head);
        assert!(note.is_none())
    }

    #[test]
    fn test_set_note() {
        let (head, repo) = init_repo_with_commit();
        let git = Git::open(&repo.path()).expect("should be able to open the repository");
        git.set_note(head, "a note").expect("not should be written");
        let note = git
            .find_note::<String>(head)
            .expect("the note has to be present");
        assert_eq!(note, "a note");
    }

    #[test]
    fn test_overwrite_note() {
        let (head, repo) = init_repo_with_commit();

        let git = Git::open(&repo.path()).expect("should be able to open the repository");
        git.set_note(head, "a note").expect("not should be written");
        git.set_note(head, "a note 2")
            .expect("not should be written");

        let note = git
            .find_note::<String>(head)
            .expect("the note has to be present");

        assert_eq!(note, "a note 2");
    }

    #[test]
    fn test_delete_note_two_times() {
        let (head, repo) = init_repo_with_commit();

        let git = Git::open(&repo.path()).expect("should be able to open the repository");

        git.delete_note(&head).expect("should work");
        git.delete_note(&head).expect("should work");
    }

    #[test]
    fn test_push_force_with_lease_refused() {
        let repo = GitTmp::init_bare("main");
        let clone = repo.clone();

        repo.new_file(
            "readme.md",
            concat!("# Star wars", "\n", "Hello there\n", "General Kenobi\n"),
        );
        repo.add_all();
        repo.commit("first commit");
        repo.new_file(
            "other_file.md",
            concat!(
                "# Pride and prejudice",
                "\n",
                "I love you. Most ardently.\n"
            ),
        );
        repo.add_all();
        repo.commit("pride and prejudice");

        // Let's open git in clone
        let git = Git::open(&clone.path()).expect("git should be open");
        // let's add a file and commit it
        clone.new_file("yolo.md", "some content");
        clone.add_all();
        clone.commit("my first commit");
        clone.push(); // To create a local remote

        // let's push force from repo
        // it will delete the history of clone
        repo.push();
        // the push force with lease should be refused because the origin has divered
        let result = git.push_force_with_lease("origin", "main");
        assert!(result.is_err());
    }

    #[test]
    fn test_push_force_with_lease_accepted() {
        let repo = GitTmp::init_bare("main");
        let clone = repo.clone();

        repo.new_file(
            "readme.md",
            concat!("# Star wars", "\n", "Hello there\n", "General Kenobi\n"),
        );
        repo.add_all();
        repo.commit("first commit");
        repo.new_file(
            "other_file.md",
            concat!(
                "# Pride and prejudice",
                "\n",
                "I love you. Most ardently.\n"
            ),
        );
        repo.add_all();
        repo.commit("pride and prejudice");
        repo.push();

        // Let's open git in clone
        let git = Git::open(&clone.path()).expect("git should be open");
        // let's add a file and commit it
        clone.pull();
        clone.amend("hello again"); // The history has been modified
        clone.new_file("anotherfile.md", "hello other file");
        clone.add_all();
        clone.commit("new commit");
        // the two origins matched, so we can erase the history
        let result = git.push_force_with_lease("origin", "main");
        assert!(result.is_ok());
    }

    #[test]
    fn test_push_force() {
        let repo = GitTmp::init_bare("main");
        let clone = repo.clone();

        repo.new_file(
            "readme.md",
            concat!("# Star wars", "\n", "Hello there\n", "General Kenobi\n"),
        );
        repo.add_all();
        repo.commit("first commit");
        repo.new_file(
            "other_file.md",
            concat!(
                "# Pride and prejudice",
                "\n",
                "I love you. Most ardently.\n"
            ),
        );
        repo.add_all();
        repo.commit("pride and prejudice");

        // Let's open git in clone
        let git = Git::open(&clone.path()).expect("git should be open");
        // let's add a file and commit it
        clone.new_file("yolo.md", "some content");
        clone.add_all();
        clone.commit("my first commit");
        clone.push(); // To create a local remote

        // let's push force from repo
        // it will delete the history of clone
        repo.push();
        // This test is based on the push_force_with_lease one
        // where push --force-with-lease fails, push --force has to work
        let result = git.push_force("origin", "main");
        assert!(result.is_ok());
    }

    // Testing `main_branch`

    /// Initializes a repository with a main branch
    fn init_main_branch_test(initial_branch: &str) -> GitTmp {
        let repo = GitTmp::init_bare(initial_branch);
        repo.new_file(
            "readme.md",
            concat!("# Star wars", "\n", "Hello there\n", "General Kenobi\n"),
        );
        repo.add_all();
        repo.commit("first commit");
        repo.push();
        repo
    }

    #[test]
    fn test_find_main_branch_main() {
        let repo = init_main_branch_test("main");
        let git = Git::open(&repo.path()).unwrap();
        let branch = git.main_branch().unwrap();
        let branch = branch.name().unwrap().unwrap();
        assert_eq!(branch, "main");
    }

    #[test]
    fn test_find_main_branch_master() {
        let repo = init_main_branch_test("master");
        let git = Git::open(&repo.path()).unwrap();
        let branch = git.main_branch().unwrap();
        let branch = branch.name().unwrap().unwrap();
        assert_eq!(branch, "master");
    }

    #[test]
    fn test_find_unknown_branch() {
        let repo = init_main_branch_test("unknown");
        let git = Git::open(&repo.path()).unwrap();
        let branch = git.main_branch();
        assert!(branch.is_none())
    }

    #[test]
    fn test_list_commits_from_main() {
        let (_, repo) = init_repo_with_commit();
        let git = Git::open(&repo.path()).unwrap();
        let commits = git.list_commits::<String>().unwrap();
        assert_eq!(commits.len(), 0) // because we are on main
    }

    #[test]
    fn test_list_commits_from_other_branch() {
        let (_, repo) = init_repo_with_commit();
        repo.create_branch("test");
        repo.new_file("hey", "hey");
        repo.add_all();
        let oid = repo.commit("first commit on my branch");

        let git = Git::open(&repo.path()).unwrap();
        let commits = git.list_commits::<String>().unwrap();
        assert_eq!(commits.len(), 1);
        let commit = commits.iter().next().unwrap();
        assert_eq!(commit.id, oid);
        assert_eq!(commit.note, None);
        assert_eq!(commit.title, "first commit on my branch");
        assert_eq!(commit.description, Some("".to_string())); // TODO: empty string should not be allowed
    }
}
