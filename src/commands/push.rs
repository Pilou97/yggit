use crate::{
    core::{apply, push_from_notes, save_note},
    git::{Editor, Git},
    parser::{commits_to_string, instruction_from_string},
};
use anyhow::{Context, Result};
use clap::Args;

#[derive(Debug, Args)]
pub struct Push {
    /// use --force to update branches,
    /// by default it is using --force-with-lease
    #[arg(short, long, default_value_t = false)]
    force: bool,
}

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

impl Push {
    pub fn execute(&self, git: Git<impl Editor>) -> Result<()> {
        let commits = git.list_commits()?;
        let output = commits_to_string(commits);
        let output = format!("{}\n{}", output, COMMENTS);

        let content = git.edit_text(output)?;

        let commits = instruction_from_string(content).context("Cannot parse instruction")?;

        save_note(&git, commits)?;
        apply(&git)?;
        push_from_notes(&git, self.force)?;

        Ok(())
    }
}
