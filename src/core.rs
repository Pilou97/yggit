use git2::Oid;
use serde::{Deserialize, Serialize};

use crate::git::{EnhancedCommit, Git};

#[derive(Serialize, Deserialize)]
pub enum Note {
    Target { branch: String },
}

/// Action is a super set of Note
#[derive(Clone)]
pub enum Action {
    Target { branch: String },
}

#[derive(Clone)]
pub struct Instruction {
    pub id: Oid,
    pub action: Option<Action>,
}

// Process instruction
// updates the notes
pub fn process_instructions(git: &Git, instructions: Vec<Instruction>) {
    for instruction in instructions {
        let Instruction { id: oid, action } = instruction;

        match action {
            Some(Action::Target { branch }) => {
                // add note
                let note = Note::Target { branch };
                let Ok(()) = git.set_note(oid, note) else {continue;};
            }
            None => {
                // delete note
                git.delete_note(&oid);
            }
        }
    }
}

/// Apply the notes
pub fn apply_notes(git: &Git) {
    let commits = git.list_commits();

    for commit in commits {
        let EnhancedCommit {
            id,
            note: Some(Note::Target { branch }),
            ..
        } = commit else {continue;};
        git.set_branch_to_commit(&branch, id).unwrap();
    }
}

/// Push force the branches with lease
pub fn push_branches(git: &Git) {
    let commits = git.list_commits();

    // Push all branch, starting by the first one
    // When a branch cannot be pushed it stops
    for commit in &commits {
        let EnhancedCommit { note: Some(Note::Target { branch }), .. } = commit else {continue;};
        let local_remote_commit = git.find_local_remote_head(branch);
        let remote_commit = git.find_remote_head(branch);
        let local_commit = git.head_of(branch);

        if local_remote_commit != remote_commit {
            println!("cannot push {}", branch);
            return;
        }

        if local_commit == remote_commit {
            println!("{} is up to date", branch);
            continue;
        }

        println!("pushing {}", branch);
        git.push_force(branch);
        println!("\r{} pushed", branch);
    }
}
