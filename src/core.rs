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
                let Ok(note) = serde_json::to_string(&note) else {continue};

                let _ = git
                    .repository
                    .note(&git.signature, &git.signature, None, oid, &note, true);
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

    for commit in commits {
        let EnhancedCommit { note, .. } = commit;
        match note {
            None => (),
            Some(Note::Target {
                branch: branch_name,
            }) => {
                git.push_force(&branch_name);

                // TODO force with lease
                // Check if the upstream has changed compared to local
                // If so do not push
                // else push (with lease)
            }
        }
    }
}
