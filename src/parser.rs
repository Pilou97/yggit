// Git related

use crate::{
    core::{Note, Push},
    git::EnhancedCommit,
};
use git2::Oid;
use pest::{iterators::Pair, Parser};
use pest_derive::Parser;

pub fn commits_to_string(commits: Vec<EnhancedCommit<Note>>) -> String {
    let mut output = String::default();
    for commit in commits {
        output = format!("{}{} {}\n", output, commit.id, commit.title);
        if let Some(Note { push, tests }) = commit.note {
            if let Some(Push { origin, branch }) = &push {
                output = format!("{}-> {}:{}\n", output, origin, branch);
            }
            for command in tests {
                output = format!("{}$ {}\n", output, command);
            }
            // An empty line is added so that is cleaner to differentiate the different MR
            if push.is_some() {
                output = format!("{}\n", output);
            }
        }
    }
    output
}

#[derive(Parser)]
#[grammar = "parser/yggit.pest"]
struct YggitParser;

#[derive(Debug, Clone)]
pub struct Target {
    pub origin: String,
    pub branch: String,
}

#[derive(Debug, Clone)]
pub struct Commit {
    pub hash: Oid,
    #[allow(dead_code)]
    pub title: String,
    pub target: Option<Target>,
    pub tests: Vec<String>,
}

fn parse_target(pair: Pair<Rule>) -> Option<Target> {
    let target = pair.into_inner();

    let mut parsed_origin = "origin".into();
    let mut parsed_branch = None;

    for pair in target.into_iter() {
        match pair.as_rule() {
            Rule::origin => {
                parsed_origin = pair.as_str().to_string();
            }
            Rule::branch_name => {
                parsed_branch = Some(pair.as_str().to_string());
            }
            _ => (),
        }
    }
    let parsed_branch = parsed_branch?;

    Some(Target {
        origin: parsed_origin,
        branch: parsed_branch,
    })
}

fn parse_test(pair: Pair<Rule>) -> String {
    pair.into_inner().next().unwrap().as_str().to_string()
}

fn parse_commit(pair: Pair<Rule>) -> Option<Commit> {
    let mut commit = pair.into_inner();

    let git_commit = commit.next()?;
    let mut git_commit = git_commit.into_inner();

    let hash = git_commit.next()?;
    let hash = Oid::from_str(hash.as_str()).ok()?;

    let title = git_commit.next()?;
    let title = title.as_str();

    let mut target = None;
    let mut tests = Vec::default();

    // Optional target
    for pair in commit {
        match pair.as_rule() {
            Rule::target => {
                target = parse_target(pair);
            }
            Rule::test => {
                let test = parse_test(pair);
                tests.push(test);
            }
            _ => (),
        }
    }

    let _ = tests;
    Some(Commit {
        hash,
        title: title.to_string(),
        target,
        tests,
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

pub fn instruction_from_string(input: String) -> Option<Vec<Commit>> {
    let pair = YggitParser::parse(Rule::commits, &input)
        .map_err(|err| println!("{err}"))
        .ok()?
        .next()?;
    let commits = parse_value(pair)?;

    Some(commits)
}
