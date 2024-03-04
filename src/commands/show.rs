use crate::{git::Git, parser::commits_to_string};
use anyhow::Result;
use clap::Args;

#[derive(Debug, Args)]
pub struct Show {}

impl Show {
    pub fn execute(&self, git: Git) -> Result<()> {
        let commits = git.list_commits()?;
        let output = commits_to_string(commits);
        println!("{}", output.trim());
        Ok(())
    }
}
