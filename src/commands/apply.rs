use crate::{
    core::{apply, save_note},
    git::Git,
    parser::{commits_to_string, instruction_from_string},
};
use anyhow::{Context, Result};
use clap::Args;

#[derive(Debug, Args)]
pub struct Apply {}

const COMMENTS: &str = r#"
# Here is how to use yggit
# 
# Commands:
# -> <branch> add a branch to the above commit
# -> <origin>:<branch> add a branch to the above commit
# 
# What happens next?
#  - All branches are pushed on origin, except if you specified a custom origin
#
# It's not a rebase, you can't edit commits nor reorder them
"#;

impl Apply {
    pub fn execute(&self, git: Git) -> Result<()> {
        let commits = git.list_commits()?;
        let output = commits_to_string(commits);

        let file_path = "/tmp/yggit";

        let output = format!("{}\n{}", output, COMMENTS);
        std::fs::write(file_path, output).context("Cannot write yggit file to filesystem")?;

        let content = git.edit_file(file_path)?;

        let commits = instruction_from_string(content).context("Cannot parse instructions")?;

        save_note(&git, commits)?;

        apply(&git)?;

        Ok(())
    }
}
