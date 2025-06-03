use git2::Oid;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use yggit_db::{Database, DatabaseError};
use yggit_git::{Git, GitError};
use yggit_parser::{Commit, Line, Parser, ParserError};
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
    db: impl Database,
    editor: impl Editor,
    force: bool,
    onto: Option<String>,
) -> Result<(), CoreError> {
    let onto = match onto {
        Some(onto) => onto,
        None => git.main().map_err(CoreError::GitError)?,
    };

    // only compatible with main (for now)
    let commits = git.list_commits(&onto).map_err(CoreError::GitError)?;

    // Now let's retrieve the branch for the existing commits
    let branches = commits
        .iter()
        .filter_map(|commit| match db.read::<Branch>(&commit.oid, "branch") {
            Ok(Some(branch)) => Some(Ok((commit.clone(), branch))),
            Ok(None) => None,
            Err(err) => Some(Err(err)),
        })
        .collect::<Result<HashMap<yggit_git::Commit, Branch>, DatabaseError>>()
        .map_err(CoreError::DatabaseError)?;

    // Let's create a string with this, so that the user can edit it
    let todo = commits
        .iter()
        .flat_map(|commit| {
            let commit_line = Line::Commit(Commit {
                sha: commit.oid.to_string(),
                title: commit.title.to_string(),
            });
            let branch_line = branches.get(&commit).map(|branch| {
                Line::Branch(yggit_parser::Branch {
                    origin: branch.origin.clone(),
                    name: branch.target.clone(),
                })
            });
            match branch_line {
                Some(branch_line) => vec![commit_line, branch_line],
                None => vec![commit_line],
            }
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
            db.delete(&commit.oid, "branch")
                .map_err(CoreError::DatabaseError)?;

            match branches.get(&commit.oid) {
                Some(branch) => {
                    db.write(&commit.oid, "branch", branch)
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
