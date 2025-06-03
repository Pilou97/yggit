use git2::Oid;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use yggit_db::{Database, DatabaseError};
use yggit_git::{Git, GitError};
use yggit_parser::{Line, Parser, ParserError};
use yggit_ui::{Editor, EditorError};

pub enum CoreError {
    GitError(GitError),
    DatabaseError(DatabaseError),
    EditorError(EditorError),
    ParserError(ParserError),
    OidParsing(String),
}

#[derive(Serialize, Deserialize)]
struct Branch {
    target: String,
    origin: Option<String>,
}

pub fn push<'a>(
    git: Git<'a>,
    db: Database<'a>,
    editor: impl Editor,
    force: bool,
) -> Result<(), CoreError> {
    // only compatible with main (for now)
    let commits = git.list_commits("main").map_err(CoreError::GitError)?;

    // Now let's retrieve the branch for the existing commits
    let _branches = commits
        .iter()
        .filter_map(|oid| match db.read::<Branch>(&oid, "branch") {
            Ok(Some(branch)) => Some(Ok((oid.clone(), branch))),
            Ok(None) => None,
            Err(err) => Some(Err(err)),
        })
        .collect::<Result<HashMap<Oid, Branch>, DatabaseError>>()
        .map_err(CoreError::DatabaseError)?;

    // Let's create a string with this, so that the user can edit it
    let todo = commits
        .iter()
        .flat_map(|_commit| {
            // TODO:
            // Modify git::list_commits to return a Commit object with Oid + Name + String
            // Modify parser to handle the string conversion
            vec![1]
        })
        .map(|line| line.to_string())
        .collect::<Vec<String>>()
        .join("\n");

    // Now the user should modify the todo (or not)
    let todo = editor.edit(todo).map_err(CoreError::EditorError)?;

    // Now we can parse it
    let lines = Parser::parse_file(&todo).map_err(CoreError::ParserError)?;

    // Now we retrieve the branches and the correspoding oid
    let branches = lines
        .windows(2)
        .filter_map(|tuple| {
            let fst = tuple.get(0).unwrap();
            let snd = tuple.get(1).unwrap();
            match (fst, snd) {
                (Line::Commit(commit), Line::Branch(branch)) => match Oid::from_str(&commit.sha) {
                    Ok(oid) => Some(Ok((
                        oid,
                        Branch {
                            target: branch.name.clone(),
                            origin: branch.origin.clone(),
                        },
                    ))),
                    Err(_) => Some(Err(CoreError::OidParsing(commit.sha.clone()))),
                },
                _ => None,
            }
        })
        .collect::<Result<HashMap<Oid, Branch>, CoreError>>()?;

    // Now we need to save the state
    commits
        .iter()
        .map(|commit| -> Result<(), CoreError> {
            db.delete(commit, "branch")
                .map_err(CoreError::DatabaseError)?;

            match branches.get(commit) {
                Some(branch) => {
                    db.write(commit, "branch", branch)
                        .map_err(CoreError::DatabaseError)?;
                    Ok(())
                }
                None => Ok(()),
            }
        })
        .collect::<Result<(), CoreError>>()?;

    // Now we can push
    branches
        .into_iter()
        .map(|(oid, branch)| -> Result<(), CoreError> {
            git.set_branch_to_commit(&branch.target, oid)
                .map_err(CoreError::GitError)?;
            let origin = branch.origin.unwrap_or("origin".to_string());

            if force {
                git.push_force_with_lease(&origin, &branch.target)
                    .map_err(CoreError::GitError)?;
            } else {
                git.push(&origin, &branch.target)
                    .map_err(CoreError::GitError)?;
            }

            Ok(())
        })
        .collect::<Result<Vec<()>, CoreError>>()?;
    Ok(())
}
