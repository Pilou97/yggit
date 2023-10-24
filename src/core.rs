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
    OnlyTests,
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
            NoteMergingPolicy::OnlyTests => {
                let new_tests = new_commit.tests;
                let mut current_note = current_note.unwrap_or_default();
                current_note.tests = new_tests;
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

/// Execute the push instructions from the notes
///
/// Change the head of the given branches
/// Push the branches to origin
pub fn push_from_notes(git: &Git) {
    let commits = git.list_commits();

    // Update the commits
    for commit in &commits {
        let EnhancedCommit {
            id,
            note:
                Some(Note {
                    push: Some(Push { target }),
                    ..
                }),
            ..
        } = commit
        else {
            continue;
        };
        // Set the head of the branch to the given commit
        git.set_branch_to_commit(target, *id).unwrap(); // TODO: manage error
    }

    // Push everything
    for commit in &commits {
        let EnhancedCommit {
            note:
                Some(Note {
                    push: Some(Push { target }),
                    ..
                }),
            ..
        } = commit
        else {
            continue;
        };

        let local_remote_commit = git.find_local_remote_head(target);
        let remote_commit = git.find_remote_head(target);
        let local_commit = git.head_of(target);

        if local_remote_commit != remote_commit {
            println!("cannot push {}", target);
            return;
        }

        if local_commit == remote_commit {
            println!("{} is up to date", target);
            continue;
        }

        println!("pushing {}", target);
        git.push_force(target);
        println!("\r{} pushed", target);
    }
}

pub fn execute_tests_from_notes(git: &Git, commits: Vec<EnhancedCommit<Note>>) -> Result<(), ()> {
    // Create a file
    let mut output = String::new();
    for commit in commits {
        let EnhancedCommit {
            id, title, note, ..
        } = commit;

        let mut commit = format!("pick {id} {title}\n");
        if let Some(Note { tests, .. }) = note {
            for test in tests {
                commit = format!("{commit}exec {test}\n")
            }
        }
        output = format!("{output}{commit}")
    }

    let main = git.main_branch().unwrap();
    git.start_rebase(main);

    std::fs::write(".git/rebase-merge/git-rebase-todo", output).expect("I don't know");

    Ok(())
}
