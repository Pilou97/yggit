mod config;
mod ui;

#[allow(clippy::module_inception)]
mod git;

pub use git::EnhancedCommit;
pub use git::Git;
pub use ui::Editor;
pub use ui::Terminal;
