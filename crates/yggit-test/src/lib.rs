use git2::Repository;
use std::fmt::Debug;
use std::io::Write;
use std::sync::Arc;
use std::{fs::File, path::Path, process::Command};
use tempfile::TempDir;

macro_rules! assert_cmd {
    ($expr:expr, $reason:expr) => {
        let result = $expr.unwrap();
        let stderr = String::from_utf8(result.stderr).unwrap_or("Error when parsing stderr".into());
        let stdout = String::from_utf8(result.stdout).unwrap_or("Error when parsing stdout".into());

        assert!(
            result.status.success(),
            "{}, stderr: \n{}\nstdout:\n {}",
            $reason,
            stderr,
            stdout
        );
    };
}

pub struct TempRepository {
    pub bare_dir: Arc<TempDir>,
    pub cloned_dir: TempDir,
    pub repository: Repository,
}

impl Debug for TempRepository {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TempRepository")
            .field("bare_dir", &self.bare_dir)
            .field("cloned_dir", &self.cloned_dir)
            .finish()
    }
}

impl AsRef<Repository> for TempRepository {
    fn as_ref(&self) -> &Repository {
        &self.repository
    }
}

impl Default for TempRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl TempRepository {
    pub fn new() -> Self {
        let bare_dir = TempDir::with_suffix(".git").expect("should be able to create bare folder");
        assert_cmd!(
            Command::new("git")
                .current_dir(bare_dir.as_ref())
                .arg("init")
                .arg("--initial-branch")
                .arg("main")
                .arg("--bare")
                .output(),
            "git init bare should work"
        );

        let cloned_dir = TempDir::new().expect("should be able to create cloned folder");
        assert_cmd!(
            Command::new("git")
                .arg("clone")
                .arg(bare_dir.as_ref())
                .arg(cloned_dir.as_ref())
                .output(),
            "git clone should work"
        );

        let repository = Repository::discover(&cloned_dir).unwrap();
        Self {
            bare_dir: Arc::new(bare_dir),
            cloned_dir,
            repository,
        }
    }

    pub fn set_identity(&self, name: &str, email: &str) {
        // git config user.email "your.email@example.com"
        assert_cmd!(
            Command::new("git")
                .current_dir(self.cloned_dir.as_ref())
                .arg("config")
                .arg("user.email")
                .arg(email)
                .output(),
            "set email should work"
        );

        // git config user.name "Your Name"
        assert_cmd!(
            Command::new("git")
                .current_dir(self.cloned_dir.as_ref())
                .arg("config")
                .arg("user.name")
                .arg(name)
                .output(),
            "set name should work"
        );
    }

    pub fn add_file(&self, filename: &str, content: &str) {
        let filepath = Path::new(&self.cloned_dir.path()).join(filename);
        let mut file = File::create(&filepath).unwrap();
        writeln!(file, "{}", content).unwrap();

        assert_cmd!(
            Command::new("git")
                .current_dir(&self.cloned_dir)
                .arg("add")
                .arg(filepath)
                .output(),
            "git add should work"
        );
    }

    pub fn commit(&self, message: &str) {
        assert_cmd!(
            Command::new("git")
                .current_dir(&self.cloned_dir)
                .arg("commit")
                .arg("-m")
                .arg(message)
                .output(),
            "git commit should work"
        );
    }

    pub fn checkout_b(&self, branch: &str) {
        assert_cmd!(
            Command::new("git")
                .current_dir(&self.cloned_dir)
                .arg("checkout")
                .arg("-b")
                .arg(branch)
                .output(),
            "git checkout should work"
        );
    }
}

impl Clone for TempRepository {
    fn clone(&self) -> Self {
        let cloned_dir = TempDir::new().expect("should be able to create cloned folder");
        assert_cmd!(
            Command::new("git")
                .arg("clone")
                .arg(self.bare_dir.as_ref().as_ref())
                .arg(cloned_dir.as_ref())
                .output(),
            "git clone should work"
        );
        let repository = Repository::discover(&cloned_dir).unwrap();

        Self {
            bare_dir: self.bare_dir.clone(),
            cloned_dir,
            repository,
        }
    }
}
