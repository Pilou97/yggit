// Git related

use git2::Oid;
use nom::{bytes::complete::tag, IResult};

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

#[derive(Clone)]
enum Line {
    Oid(Oid),
    Action(Action),
}

fn parse_line(line: &str) {
    let res: IResult<&str, &str> = tag("->")(line);
    let (remaining, tag) = res.unwrap();
    println!("remaining: {}, tag: {}", remaining, tag);
}

pub fn instruction_from_string(string: String) -> Vec<Instruction> {
    let lines = string.split('\n');

    let mut before = None;
    let mut instructions = Vec::default();
    for line in lines {
        parse_line(line);

        if line.starts_with('#') {
            continue;
        }

        let current_line = match line.starts_with("->") {
            true => {
                let branch = line.chars().skip(2).collect::<String>().trim().to_string();
                Some(Line::Action(Action::Target { branch }))
            }
            false => {
                let mut iter = line.split(' ');
                let oid = iter.next();
                if let Some(oid) = oid {
                    Oid::from_str(oid).ok().map(Line::Oid)
                } else {
                    None
                }
            }
        };

        let instruction = match (before, current_line.clone()) {
            (None, None) => None,
            (None, Some(_)) => None,
            (Some(Line::Oid(oid)), None) => Some(Instruction {
                id: oid,
                action: None,
            }),
            (Some(Line::Oid(oid)), Some(Line::Action(Action::Target { branch }))) => {
                Some(Instruction {
                    id: oid,
                    action: Some(Action::Target { branch }),
                })
            }
            (Some(Line::Action(_)), None) => None,
            (Some(Line::Action(_)), Some(Line::Oid(_))) => None,
            (Some(Line::Oid(oid)), Some(Line::Oid(_))) => Some(Instruction {
                id: oid,
                action: None,
            }),
            (Some(Line::Action(_)), Some(Line::Action(_))) => None,
        };

        before = current_line;
        if let Some(instruction) = instruction {
            instructions.push(instruction)
        }
    }

    instructions
}
