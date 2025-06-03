use std::{
    borrow::Cow,
    sync::{Arc, Mutex},
};

use auth_git2::GitAuthenticator;
use git2::{Error, Oid, Repository};
use thiserror::Error;

/// A git client
pub struct Git<'a> {
    repository: &'a Repository,
    auth: GitAuthenticator,
}

enum PushMode {
    Normal,
    Force,
    ForceWithLease,
}

#[derive(Debug)]
pub enum NegotiationResult {
    NoPushNeeded,
    RemoteDiverged,
    AllowedToPush,
    AllowedToPushNewBranch,
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
    #[error("Config not found")]
    ConfigNotFound,
    #[error("Remote {0} not found")]
    RemoteNotFound(String),
    #[error("Branch {branch} wasn't pushed on {origin}")]
    NotPushed {
        branch: String,
        origin: String,
        reason: Cow<'static, str>,
    },
    #[error("Feature {0} is not yet implemented")]
    NotYetImplemented(&'static str),
    #[error("Commit {0} was not found")]
    CommitNotFound(Oid),
}

impl<'a> Git<'a> {
    pub fn new(repository: &'a Repository) -> Self {
        Git {
            repository,
            auth: GitAuthenticator::new(),
        }
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

    /// Simple push
    /// Returns Ok(()) if the push was not needed
    fn custom_push(&self, origin: &str, branch: &str, mode: PushMode) -> Result<(), GitError> {
        let git_config = self
            .repository
            .config()
            .map_err(|_| GitError::ConfigNotFound)?;
        let mut push_options = git2::PushOptions::new();

        let mut remote_callbacks = git2::RemoteCallbacks::new();
        remote_callbacks.credentials(self.auth.credentials(&git_config));

        let fetch_refname = match &mode {
            PushMode::Normal => format!("refs/heads/{branch}"),
            PushMode::Force => format!("+refs/heads/{branch}"),
            PushMode::ForceWithLease => format!("refs/heads/{branch}"),
        };

        let mut remote = self
            .repository
            .find_remote(origin)
            .map_err(|_| GitError::RemoteNotFound(origin.to_string()))?;

        let negotiation_result = Arc::new(Mutex::new(None));
        let negotiation_result_read = Arc::clone(&negotiation_result);
        match mode {
            PushMode::Normal | PushMode::Force => {
                remote_callbacks.push_negotiation(move |remote_updates| {
                    let mut negotiation_result = negotiation_result.lock().unwrap();
                    let Some(remote_update) = remote_updates.iter().next() else {
                        *negotiation_result = Some(NegotiationResult::NoPushNeeded);
                        return Err(Error::from_str("not updates to be done"));
                    };

                    if remote_update.src() == git2::Oid::zero() {
                        *negotiation_result = Some(NegotiationResult::AllowedToPushNewBranch);
                        return Ok(());
                    }

                    *negotiation_result = Some(NegotiationResult::AllowedToPush);
                    Ok(())
                });
            }
            PushMode::ForceWithLease => {
                remote_callbacks.push_negotiation(move |remote_updates| {
                    let null = git2::Oid::zero();
                    let mut negotiation_result = negotiation_result.lock().unwrap();
                    let Some(remote_update) = remote_updates.iter().next() else {
                        *negotiation_result = Some(NegotiationResult::NoPushNeeded);
                        return Err(Error::from_str("not updates to be done"));
                    };

                    if remote_update.src() == null {
                        *negotiation_result = Some(NegotiationResult::AllowedToPushNewBranch);
                        return Ok(());
                    }

                    // Comparing src with local origin
                    let remote_origin_oid = remote_update.src();
                    // Get the head of this branch
                    let local_origin_oid = {
                        let local_origin_name = remote_update
                            .src_refname()
                            .ok_or(Error::from_str("cannot parse source refname"))?;
                        let upstream_name = local_origin_name
                            .strip_prefix("refs/heads/")
                            .ok_or(Error::from_str("cannot strip local origin name"))?;
                        self.repository
                            .find_reference(&format!("refs/remotes/{}/{}", origin, upstream_name))
                            .ok()
                            .and_then(|reference| reference.peel_to_commit().ok())
                            .map(|commit| commit.id())
                            .ok_or(Error::from_str("cannot find the commit reference hash"))?
                    };
                    if remote_origin_oid == local_origin_oid {
                        *negotiation_result = Some(NegotiationResult::AllowedToPush);
                        Ok(())
                    } else {
                        *negotiation_result = Some(NegotiationResult::RemoteDiverged);
                        Err(Error::from_str("Origins have divered"))
                    }
                });
            }
        };
        push_options.remote_callbacks(remote_callbacks);
        let push_res = remote.push(&[fetch_refname], Some(&mut push_options));

        let negotiation_result = negotiation_result_read.lock().unwrap();
        let negotiation_result = negotiation_result.as_ref().unwrap();

        match (negotiation_result, push_res) {
            (NegotiationResult::NoPushNeeded, _) => Ok(()),
            (NegotiationResult::RemoteDiverged, _) => Err(GitError::NotPushed {
                branch: branch.to_string(),
                origin: origin.to_string(),
                reason: "the origin on the server is not the same as the local one".into(),
            }),
            (NegotiationResult::AllowedToPush, Err(err)) => Err(GitError::NotPushed {
                branch: branch.to_string(),
                origin: origin.to_string(),
                reason: err.message().to_string().into(),
            }),
            (NegotiationResult::AllowedToPushNewBranch, Err(err)) => Err(GitError::NotPushed {
                branch: branch.to_string(),
                origin: origin.to_string(),
                reason: err.message().to_string().into(),
            }),
            (NegotiationResult::AllowedToPush, Ok(()))
            | (NegotiationResult::AllowedToPushNewBranch, Ok(())) => Ok(()),
        }
    }

    /// Equivalent of `git push --force-with-lease`
    pub fn push_force_with_lease(&self, origin: &str, branch: &str) -> Result<(), GitError> {
        self.custom_push(origin, branch, PushMode::ForceWithLease)
    }

    /// Equivalent of `git push --force`
    pub fn push_force(&self, origin: &str, branch: &str) -> Result<(), GitError> {
        self.custom_push(origin, branch, PushMode::Force)
    }

    /// Equivalent of `git push`
    pub fn push(&self, origin: &str, branch: &str) -> Result<(), GitError> {
        self.custom_push(origin, branch, PushMode::Normal)
    }

    /// Set a branch to a given commit
    pub fn set_branch_to_commit(&self, branch: &str, oid: Oid) -> Result<(), GitError> {
        let commit = self
            .repository
            .find_commit(oid)
            .map_err(|_| GitError::CommitNotFound(oid))?;

        self.repository
            .branch(branch, &commit, true)
            .map_err(|_| GitError::BranchNotFound(branch.to_string()))?;

        Ok(())
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
