use std::process::Command;

use crate::{
    core::{apply_notes, process_instructions, push_branches},
    git::Git,
    parser::{commits_to_string, instruction_from_string},
};

/// Push the branches
///
/// Display the interface to create branches
/// Create branches
/// Push force with lease branches
pub fn push() {
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
