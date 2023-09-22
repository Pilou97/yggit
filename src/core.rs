use crate::git::{EnhancedCommit, Git};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct Push {
    pub target: String,
}

#[derive(Deserialize, Serialize)]
pub struct Note {
    pub push: Option<Push>,
    pub tests: Vec<String>,
}

/// Save the note to the commit
///
/// Also deletes note if there is nothing new
pub fn save_note(git: &Git, commits: Vec<crate::parser::Commit>) {
    for commit in commits {
        // Extract information from commit
        let crate::parser::Commit {
            hash,
            target,
            tests,
            ..
        } = commit;

        let is_empty = target.is_none() && tests.is_empty();

        if is_empty {
            git.delete_note(&hash);
        } else {
            // Create the note
            let note = Note {
                push: target.map(|target| Push { target }),
                tests: tests,
            };

            // Save the note
            git.set_note(hash, note).unwrap();
        }
    }
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
        } = commit else {continue;};
        // Set the head of the branch to the given commit
        git.set_branch_to_commit(&target, *id).unwrap(); // TODO: manage error
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
        } = commit else {continue;};

        let local_remote_commit = git.find_local_remote_head(&target);
        let remote_commit = git.find_remote_head(&target);
        let local_commit = git.head_of(&target);

        if local_remote_commit != remote_commit {
            println!("cannot push {}", target);
            return;
        }

        if local_commit == remote_commit {
            println!("{} is up to date", target);
            continue;
        }

        println!("pushing {}", target);
        git.push_force(&target);
        println!("\r{} pushed", target);
    }
}

pub fn execute_tests_from_notes(git: &Git) -> Result<(), ()> {
    let main = git
        .repository
        .find_branch("main", git2::BranchType::Local)
        .unwrap();

    git.rebase(main, |oid, git| {
        let commit = git.find_commit::<Note>(oid).unwrap();

        let note = match commit.note {
            None => return Ok(()),
            Some(note) => note,
        };

        for command in note.tests {
            let status = std::process::Command::new("sh")
                .arg("-c")
                .arg(&command)
                .status();

            match status {
                Ok(status) => match status.success() {
                    true => {
                        continue;
                    }
                    false => {
                        println!("{} :failed", &command);
                        return Err(());
                    }
                },
                Err(_) => {
                    println!("{} :failed", &command);
                    return Err(());
                }
            }
        }
        Ok(())
    })
}
