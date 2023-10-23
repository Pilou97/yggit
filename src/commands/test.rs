use crate::{
    core::{execute_tests_from_notes, merge_notes, NoteMergingPolicy},
    git::Git,
    parser::{commits_to_string, instruction_from_string, UiFilter},
};
use clap::Args;

use super::Execute;

#[derive(Debug, Args)]
pub struct Test {}

const COMMENTS: &str = r#"
# Here is how to use yggit
# 
# Commands:
# -> <branch> add a branch to the above commit
# $ <command> this command will be executed 
# 
# What happens next?
#  - All branches are pushed
#
# It's not a rebase, you can't edit commits nor reorder them
"#;

impl Execute for Test {
    fn execute(&self) -> Result<(), ()> {
        let git = Git::open(".");

        let commits = git.list_commits();
        let output = commits_to_string(commits, UiFilter::OnlyTests);

        let file_path = "/tmp/yggit";

        let output = format!("{}\n{}", output, COMMENTS);
        std::fs::write(file_path, output).map_err(|_| println!("cannot write file to disk"))?;

        let content = git.edit_file(file_path)?;

        let commits = instruction_from_string(content).ok_or_else(|| {
            println!("Cannot parse instructions");
        })?;

        merge_notes(&git, commits, NoteMergingPolicy::OnlyTests);

        execute_tests_from_notes(&git)?;

        Ok(())
    }
}
