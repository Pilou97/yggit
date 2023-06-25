use crate::core::apply_notes;
use crate::core::process_instructions;
use crate::core::push_branches;
use crate::parser::commits_to_string;
use git::Git;
use parser::instruction_from_string;
use std::process::Command;

mod core;
mod git;
mod parser;

fn main() {
    let git = Git::open(".");

    let commits = git.list_commits();
    let output = commits_to_string(commits);

    let file = "/tmp/yggit";

    let comments = r#"
# Here is how to use yggit
# 
# Commands:
# -> <branch> add a branch to the above commit
# 
# What happens next?
#  - All branches are pushed
#
# It's not a rebase, you can't edit commits nor reorder them
# "#;

    let output = format!("{}\n{}", output, comments);

    std::fs::write(file, output).unwrap();

    let output = Command::new("nvim")
        .arg(file)
        .status()
        .expect("Failed to execute command");
    let true = output.success() else {return;};
    let file = std::fs::read_to_string(file).unwrap();

    let instructions = instruction_from_string(file);

    process_instructions(&git, instructions);

    // updates branches
    apply_notes(&git);

    // push
    push_branches(&git);
}
