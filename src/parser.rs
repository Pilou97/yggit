// Git related

use git2::Oid;

use crate::{
    core::{Action, Instruction},
    git::{EnhancedCommit, Note},
};

pub fn commits_to_string(commits: Vec<EnhancedCommit>) -> String {
    let mut output = String::default();
    for commit in commits {
        output = format!("{}{} {}\n", output, commit.id, commit.message.trim());
        if let Some(Note::Target { branch }) = commit.note {
            output = format!("{}-> {}\n", output, branch);
        }
    }
    output
}

#[derive(Clone)]
enum Line {
    Oid(Oid),
    Action(Action),
}

pub fn instruction_from_string(string: String) -> Vec<Instruction> {
    let lines = string.split('\n');

    let mut before = None;
    let mut instructions = Vec::default();
    for line in lines {
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
