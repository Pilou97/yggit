mod config;

#[allow(clippy::module_inception)]
mod git;

pub use git::EnhancedCommit;
pub use git::Git;
pub use git::Note;
