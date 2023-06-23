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

    std::fs::write("/tmp/yggit", output).unwrap();

    let output = Command::new("nvim")
        .arg("/tmp/yggit")
        .status()
        .expect("Failed to execute command");
    let true = output.success() else {return;};
    let file = std::fs::read_to_string("/tmp/yggit").unwrap();

    let instructions = instruction_from_string(file);

    process_instructions(&git.repository, &git.signature, instructions);

    // updates branches
    apply_notes(&git);

    // push
    push_branches(&git);
}
