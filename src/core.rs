use git2::{Oid, Repository, Signature};
use serde::{Deserialize, Serialize};

use crate::parser::list_commits;

#[derive(Clone)]
pub struct EnhancedCommit {
    pub id: Oid,
    pub message: String,
    pub note: Option<Note>,
}

#[derive(Clone, Serialize, Deserialize)]
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
pub fn process_instructions(
    repository: &Repository,
    signature: &Signature,
    instructions: Vec<Instruction>,
) {
    for instruction in instructions {
        let Instruction { id: oid, action } = instruction;

        match action {
            Some(Action::Target { branch }) => {
                // add note
                let note = Note::Target { branch };
                let Ok(note) = serde_json::to_string(&note) else {continue};

                let _ = repository.note(signature, signature, None, oid, &note, true);
            }
            None => {
                // delete note
                let _ = repository.note_delete(oid, None, signature, signature);
            }
        }
    }
}

/// Apply the notes
pub fn apply_notes(repository: &Repository, _signature: &Signature) {
    let commits = list_commits(repository);

    for commit in commits {
        let EnhancedCommit { id, note, .. } = commit;
        match note {
            None => (),
            Some(Note::Target { branch }) => {
                let commit = repository.find_commit(id).unwrap();

                let _ = repository.branch(&branch, &commit, true).unwrap();
            }
        }
    }
}

/// Push the branches
pub fn push_branches(repository: &Repository, _signature: &Signature) {
    let commits = list_commits(repository);
    let mut remote = repository.find_remote("origin").unwrap();

    for commit in commits {
        let EnhancedCommit { note, .. } = commit;
        match note {
            None => (),
            Some(Note::Target {
                branch: branch_name,
            }) => {
                let fetch_refname = format!("refs/heads/{}", branch_name);
                remote.connect(git2::Direction::Push).unwrap();
                remote.push(&[format!("+{}", fetch_refname)], None).unwrap();

                // TODO force with lease
                // Check if the upstream has changed compared to local
                // If so do not push
                // else push (with lease)
            }
        }
    }
}
