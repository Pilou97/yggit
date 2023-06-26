use crate::git::{EnhancedCommit, Git, Note};
use git2::Oid;

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

    // Check all the branch
    for commit in &commits {
        let EnhancedCommit { note: Some(Note::Target { branch }), .. } = commit else {continue;};
        match git.with_lease(branch) {
            Ok(_) => (),
            Err(_) => {
                println!("cannot push {}", branch);
                return;
            }
        }
    }

    // Push all the branch
    for commit in &commits {
        let EnhancedCommit { note: Some(Note::Target { branch }), .. } = commit else {continue;};
        git.push_force(branch);
    }
}
