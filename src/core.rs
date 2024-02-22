use crate::{
    git::{EnhancedCommit, Git},
    parser::Target,
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct Push {
    pub origin: Option<String>,
    pub branch: String,
}

#[derive(Deserialize, Serialize)]
pub struct Note {
    pub push: Option<Push>,
}

/// Save the note to the commit
///
/// Also deletes note if there is nothing new
pub fn save_note(git: &Git, commits: Vec<crate::parser::Commit>) {
    for commit in commits {
        // Extract information from commit
        let crate::parser::Commit { hash, target, .. } = commit;

        let is_empty = target.is_none();

        if is_empty {
            git.delete_note(&hash);
        } else {
            // Create the note
            let note = Note {
                push: target.map(|Target { origin, branch }| Push { origin, branch }),
            };

            // Save the note
            git.set_note(hash, note).unwrap();
        }
    }
}

/// Execute the instructions from the notes
/// to change the head of the given branches
pub fn apply(git: &Git) {
    let commits = git.list_commits();

    // Update the commits
    for commit in &commits {
        let EnhancedCommit {
            id,
            note:
                Some(Note {
                    push: Some(Push { branch, origin: _ }),
                    ..
                }),
            ..
        } = commit
        else {
            continue;
        };
        // Set the head of the branch to the given commit
        git.set_branch_to_commit(branch, *id).unwrap(); // TODO: manage error
    }
}

/// Push the branches to origin
pub fn push_from_notes(git: &Git) {
    let commits = git.list_commits();
    // Push everything
    for commit in &commits {
        let EnhancedCommit {
            note:
                Some(Note {
                    push: Some(Push { origin, branch }),
                    ..
                }),
            ..
        } = commit
        else {
            continue;
        };

        let origin = origin
            .clone()
            .unwrap_or(git.config.yggit.default_upstream.clone());

        git.push_force_with_lease(&origin, branch);
    }
}
