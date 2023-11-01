use crate::git::{EnhancedCommit, Git};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, PartialEq)]
pub struct Push {
    pub target: String,
}

/// Note stored in each commit
#[derive(Deserialize, Serialize, Default, PartialEq)]
pub struct Note {
    pub push: Option<Push>,
    pub tests: Vec<String>,
}

impl Note {
    /// Check if the note is empty
    fn is_empty(&self) -> bool {
        self == &Self::default()
    }
}

pub enum NoteMergingPolicy {
    OnlyTarget,
}

pub fn merge_notes(
    git: &Git,
    new_commits: Vec<crate::parser::Commit>,
    merging_policy: NoteMergingPolicy,
) -> Vec<EnhancedCommit<Note>> {
    let mut commits = Vec::default();
    for new_commit in new_commits {
        let oid = new_commit.hash;
        let EnhancedCommit {
            note: current_note, ..
        }: EnhancedCommit<Note> = git.find_commit(oid).expect("to exist");

        let next_note = match merging_policy {
            NoteMergingPolicy::OnlyTarget => {
                let new_target = new_commit.target;
                let mut current_note = current_note.unwrap_or_default();
                match new_target {
                    Some(new_target) => current_note.push = Some(Push { target: new_target }),
                    None => current_note.push = None,
                }
                current_note
            }
        };

        // Check if empty
        if next_note.is_empty() {
            git.delete_note(&oid);
        } else {
            git.set_note(oid, next_note).expect("not should be written");
        }

        let commit = git.find_commit(oid).unwrap();
        commits.push(commit);
    }
    commits
}
