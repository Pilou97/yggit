use git2::{Oid, Repository};
use thiserror::Error;

/// A git client
pub struct Git<'a> {
    repository: &'a Repository,
}

#[derive(Debug, Error)]
pub enum GitError {
    #[error("No main branch was found")]
    NoMainBranch,
    #[error("The branch [{0}] was not found")]
    BranchNotFound(String),
    #[error("Commit of branch [{0}] was not found")]
    CommitOfBranchNotFound(String),
    #[error("Cannot list commits")]
    CannotListCommit,
    #[error("Oid is not valid")]
    InvalidOid,
    #[error("Head is not present")]
    HeadNotPresent,
    #[error("Cannot reach {0} from {0}")]
    CannotReach(Oid, Oid),
}

impl<'a> Git<'a> {
    pub fn new(repository: &'a Repository) -> Self {
        Git { repository }
    }

    /// Returns the main branch
    pub fn main(&self) -> Result<String, GitError> {
        if let Ok(head) = self.repository.find_reference("refs/remotes/origin/HEAD") {
            if let Some(target) = head.symbolic_target() {
                if let Some(branch_name) = target.rsplit('/').next() {
                    return Ok(branch_name.to_string());
                }
            }
        }
        return Err(GitError::NoMainBranch);
    }

    pub fn list_commits(&self, until: &str) -> Result<Vec<Oid>, GitError> {
        let head = self
            .repository
            .head()
            .map_err(|_| GitError::HeadNotPresent)?
            .target()
            .ok_or(GitError::HeadNotPresent)?;

        let until_branch = self
            .repository
            .find_branch(until, git2::BranchType::Local)
            .map_err(|_| GitError::BranchNotFound(until.to_string()))?;

        let until_commit = until_branch
            .get()
            .peel_to_commit()
            .map_err(|_| GitError::CommitOfBranchNotFound(until.to_string()))?;

        // make sure until is a parent of HEAD
        if !self
            .repository
            .graph_descendant_of(head, until_commit.id())
            .map_err(|_| GitError::CannotReach(until_commit.id(), head))?
        {
            return Err(GitError::CannotReach(until_commit.id(), head));
        }

        let mut revwalk = self
            .repository
            .revwalk()
            .map_err(|_| GitError::CannotListCommit)?;
        revwalk
            .push_head()
            .map_err(|_| GitError::CannotListCommit)?;

        let mut commits = Vec::default();

        for oid in revwalk {
            let oid = oid.map_err(|_| GitError::InvalidOid)?;

            if oid == until_commit.id() {
                break;
            }

            // The commit has to be found, because it's listed from the revwalk
            commits.push(oid);
        }

        Ok(commits)
    }
}

#[cfg(test)]
mod tests {
    use git2::Repository;

    use crate::{Git, GitError};

    #[test]
    fn test_main_branch() {
        let repository = Repository::open("../../").unwrap();
        let git = Git::new(&repository);
        assert_eq!(git.main().unwrap(), "main");
    }

    #[test]
    fn test_list_commits() {
        let repository = Repository::open("../../").unwrap();
        let git = Git::new(&repository);
        git.list_commits("main").expect("to work");
    }

    #[test]
    fn test_list_commits_unknown_branch() {
        let repository = Repository::open("../../").unwrap();
        let git = Git::new(&repository);
        let Err(GitError::BranchNotFound(branch_name)) = git.list_commits("whouhouhou") else {
            panic!("expecting an error")
        };
        assert_eq!(branch_name, "whouhouhou")
    }
}
