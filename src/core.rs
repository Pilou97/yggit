use git2::{Oid, Repository, Signature};

use crate::git::{EnhancedCommit, Git, Note};

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
pub fn apply_notes(git: &Git) {
    let commits = git.list_commits();

    for commit in commits {
        let EnhancedCommit { id, note, .. } = commit;
        match note {
            None => (),
            Some(Note::Target { branch }) => {
                let commit = git.repository.find_commit(id).unwrap();

                let _ = git.repository.branch(&branch, &commit, true).unwrap();
            }
        }
    }
}

/// Push the branches
pub fn push_branches(git: &Git) {
    let commits = git.list_commits();
    let mut remote = git.repository.find_remote("origin").unwrap();

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
