use crate::core::apply_notes;
use crate::core::process_instructions;
use crate::core::push_branches;
use crate::parser::commits_to_string;
use git2::{Repository, Signature};
use parser::{instruction_from_string, list_commits};
use std::process::Command;

mod core;
mod parser;

fn main() {
    let repository = Repository::open(".").unwrap();
    let signature = Signature::now("Pierre-Louis", "pierrelouis.dubois@tutanota.com").unwrap();

    let commits = list_commits(&repository);
    let output = commits_to_string(commits);

    std::fs::write("/tmp/yggit", output).unwrap();

    let output = Command::new("nvim")
        .arg("/tmp/yggit")
        .status()
        .expect("Failed to execute command");
    let true = output.success() else {return;};
    let file = std::fs::read_to_string("/tmp/yggit").unwrap();

    let instructions = instruction_from_string(file);

    process_instructions(&repository, &signature, instructions);

    // updates branches
    apply_notes(&repository, &signature);

    // push
    push_branches(&repository, &signature);
}
