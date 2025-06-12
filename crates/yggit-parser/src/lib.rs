use std::fmt::Display;

use thiserror::Error;

#[derive(Debug, PartialEq, Eq)]
pub struct Commit {
    pub sha: String,
    pub title: String,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Branch {
    pub origin: Option<String>,
    pub name: String,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Line {
    Commit(Commit),
    Branch(Branch),
}

#[derive(pest_derive::Parser)]
#[grammar = "parser.pest"]
pub struct Parser;

#[derive(Debug, Error)]
pub enum ParserError {
    #[error("Unknown error happened")]
    Unknown,
    #[error("Should be a line")]
    IsNotLine { line: String },
    #[error("The file is not correct")]
    IsNotFile,
    #[error("Token expected")]
    TokenExpected,
    #[error("Invalid token")]
    InvalidToken,
}

macro_rules! as_str {
    ($pair:expr, $rule:expr) => {
        match $pair {
            Some(pair) => {
                if pair.as_rule() == $rule {
                    pair.as_str().to_string()
                } else {
                    return Err(ParserError::InvalidToken);
                }
            }
            _ => return Err(ParserError::TokenExpected),
        }
    };
}

impl Parser {
    pub fn parse_file(file: &str) -> Result<Vec<Line>, ParserError> {
        use pest::Parser;
        let pairs = Self::parse(Rule::file, file).map_err(|_| ParserError::IsNotFile)?;

        let mut file = vec![];
        for pair in pairs {
            pair.as_str().to_string();
            match pair.as_rule() {
                Rule::file => {
                    for line in pair.into_inner() {
                        let line = match line.as_rule() {
                            Rule::git_commit => {
                                let mut commit = line.into_inner();
                                let sha = as_str!(commit.next(), Rule::commit_hash);
                                let title = as_str!(commit.next(), Rule::commit_title);
                                Line::Commit(Commit { sha, title })
                            }
                            Rule::branch => {
                                let mut branch = line.into_inner();
                                let origin_or_name = branch.next();
                                let (origin, name) =
                                    match origin_or_name.as_ref().map(|pair| pair.as_rule()) {
                                        Some(Rule::origin) => {
                                            let origin = origin_or_name
                                                .map(|pair| pair.as_str().to_string());
                                            let name = as_str!(branch.next(), Rule::branch_name);
                                            (origin, name)
                                        }
                                        Some(Rule::branch_name) => {
                                            let name = origin_or_name.unwrap().as_str().to_string();
                                            (None, name)
                                        }
                                        _ => return Err(ParserError::InvalidToken),
                                    };
                                Line::Branch(Branch { origin, name })
                            }
                            Rule::EOI => continue,
                            Rule::comment => continue, // for now we ignore the comments
                            _ => return Err(ParserError::InvalidToken),
                        };
                        file.push(line);
                    }
                }
                _ => {
                    return Err(ParserError::IsNotLine {
                        line: pair.as_str().to_string(),
                    });
                }
            }
        }
        Ok(file)
    }
}

impl Display for Line {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Line::Commit(commit) => write!(f, "{} {}", commit.sha, commit.title),
            Line::Branch(branch) => match &branch.origin {
                Some(origin) => write!(f, "-> {}:{}", origin, branch.name),
                None => write!(f, "-> {}", branch.name),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{Branch, Commit, Line, Parser};

    #[test]
    fn test_parser_roundtrip() {
        let file = format!(
            "
afd1ebed7162bc404e0cc169d25fb4b01806eb2c chore: upgrade&é\"'(§è!çà) rust toolchain
afd1ebed7162bc404e0cc169d25fb4b01806eb2c chore: upgrade rust toolchain

afd1ebed7162bc404e0cc169d25fb4b01806eb2c chore: upgrade rust toolchain
-> awesome

afd1ebed7162bc404e0cc169d25fb4b01806eb2c chore: upgrade rust toolchain
-> origin:awesome
"
        );

        let lines = Parser::parse_file(&file).expect("it should be parsed");
        assert_eq!(
            lines,
            vec![
                Line::Commit(Commit {
                    sha: "afd1ebed7162bc404e0cc169d25fb4b01806eb2c".into(),
                    title: "chore: upgrade&é\"'(§è!çà) rust toolchain".into()
                }),
                Line::Commit(Commit {
                    sha: "afd1ebed7162bc404e0cc169d25fb4b01806eb2c".into(),
                    title: "chore: upgrade rust toolchain".into()
                }),
                Line::Commit(Commit {
                    sha: "afd1ebed7162bc404e0cc169d25fb4b01806eb2c".into(),
                    title: "chore: upgrade rust toolchain".into()
                }),
                Line::Branch(Branch {
                    origin: None,
                    name: "awesome".into(),
                }),
                Line::Commit(Commit {
                    sha: "afd1ebed7162bc404e0cc169d25fb4b01806eb2c".into(),
                    title: "chore: upgrade rust toolchain".into()
                }),
                Line::Branch(Branch {
                    name: "awesome".into(),
                    origin: Some("origin".to_string())
                }),
            ]
        )
    }
}
