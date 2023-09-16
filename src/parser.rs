// Git related

use crate::{
    core::{Action, Instruction, Note},
    git::EnhancedCommit,
};
use git2::Oid;
use pest::{iterators::Pair, Parser};
use pest_derive::Parser;

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

#[derive(Parser)]
#[grammar = "parser/yggit.pest"]
struct YggitParser;

#[derive(Debug, Clone)]
struct Commit {
    hash: Oid,
    #[allow(dead_code)]
    title: String,
    target: Option<String>,
}

fn parse_commit(pair: Pair<Rule>) -> Option<Commit> {
    let mut commit = pair.into_inner();

    let git_commit = commit.next()?;
    let mut git_commit = git_commit.into_inner();

    let hash = git_commit.next()?;
    let hash = Oid::from_str(hash.as_str()).ok()?;

    let title = git_commit.next()?;
    let title = title.as_str();

    // Optional target
    let target = commit.next();
    let target = match target {
        None => None,
        Some(target) => {
            let mut target = target.into_inner();
            let _ = target.next()?;

            let branch_name = target.next()?;
            Some(branch_name.as_str().to_string())
        }
    };

    Some(Commit {
        hash,
        title: title.to_string(),
        target,
    })
}

fn parse_value(pair: Pair<Rule>) -> Option<Vec<Commit>> {
    match pair.as_rule() {
        Rule::commits => {
            let mut commits = Vec::default();
            for pair in pair.into_inner() {
                let commit = parse_commit(pair)?;
                commits.push(commit);
            }
            Some(commits)
        }
        _ => None,
    }
}

pub fn instruction_from_string(input: String) -> Option<Vec<Instruction>> {
    let pair = YggitParser::parse(Rule::commits, &input).ok()?.next()?;
    let commits = parse_value(pair)?;

    let commits = commits
        .iter()
        .cloned()
        .map(|commit| Instruction {
            id: commit.hash,
            action: commit
                .target
                .map(|target| Action::Target { branch: target }),
        })
        .collect();
    Some(commits)
}
