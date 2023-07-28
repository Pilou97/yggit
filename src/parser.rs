// Git related

use git2::Oid;
use nom::{
    bytes::complete::{is_a, tag, take_till1, take_while1},
    IResult,
};

use crate::{
    core::{Action, Instruction, Note},
    git::EnhancedCommit,
};

pub fn commits_to_string(commits: Vec<EnhancedCommit<Note>>) -> String {
    let mut output = String::default();
    for commit in commits {
        output = format!("{}{} {}\n", output, commit.id, commit.title);
        if let Some(Note::Target { branch }) = commit.note {
            // An empty line is added so that is cleaner to differentiate the different branch
            output = format!("{}-> {}\n\n", output, branch);
        }
    }
    output
}

#[derive(Clone, Debug)]
enum Line {
    Oid(Oid),
    Action(Action),
    Comment,
}

fn parse_line(line: &str) -> Line {
    // First symbol
    let push_ready: IResult<&str, &str> = tag("->")(line);
    let push_draft: IResult<&str, &str> = tag("->?")(line);
    let comment: IResult<&str, &str> = tag("#")(line);
    let commit_hash: IResult<&str, &str> = is_a("1234567890abcdef")(line); // Add length

    match (push_ready, push_draft, comment, commit_hash) {
        (Ok((remaining, _)), _, _, _) => {
            println!("push_ready");

            // Remove the white characters
            let res: IResult<&str, &str> = take_while1(|c| c == ' ')(remaining);
            let (remaining, _) = res.unwrap();

            // Extract the branch name
            let res: IResult<&str, &str> = take_till1(|c| c == ' ' || c == '\n')(remaining);
            let (_, branch) = res.unwrap();

            // Returns the action
            println!("branch: {}", branch);
            Line::Action(Action::Target {
                branch: branch.to_string(),
            })
        }
        (_, Ok((_remaining, _)), _, _) => {
            println!("push_draft");
            todo!("push draft is not yet implemented")
        }
        (_, _, Ok((_remaining, _)), _) => Line::Comment,
        (_, _, _, Ok((_remaining, commit_oid))) => {
            let oid = Oid::from_str(commit_oid).unwrap();
            Line::Oid(oid)
        }
        _ => Line::Comment,
    }
}

pub fn instruction_from_string(string: String) -> Vec<Instruction> {
    let lines: Vec<Line> = string.split('\n').map(parse_line).collect();
    let current_items = lines.iter();

    let a = vec![None];
    let previous_items = a
        .iter()
        .cloned()
        .chain(lines.iter().map(|line| Some(line.clone())));

    previous_items
        .zip(current_items)
        .filter_map(|(previous, current)| match (previous, current) {
            (Some(Line::Oid(id)), Line::Action(Action::Target { branch })) => Some(Instruction {
                id: id.clone(),
                action: Some(Action::Target {
                    branch: branch.into(),
                }),
            }),
            (_, Line::Oid(id)) => Some(Instruction {
                id: id.clone(),
                action: None,
            }),
            (None, _) => None,
            (_, Line::Comment) => None,
            (Some(Line::Action(_)), Line::Action(_)) => None,
            (Some(Line::Comment), _) => None,
        })
        .collect()
}
