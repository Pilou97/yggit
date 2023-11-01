use crate::{
    core::{merge_notes, Note, NoteMergingPolicy},
    git::Git,
    parser::{commits_to_string, instruction_from_string, UiFilter},
};
use clap::Args;
use git2::Oid;

use super::Execute;

#[derive(Debug, Args)]
pub struct Push {}

const COMMENTS: &str = r#"
# Here is how to use yggit
# 
# Commands:
# -> <branch> add a branch to the above commit
# 
# What happens next?
#  - All branches are pushed
#
# It's not a rebase, you can't edit commits nor reorder them
"#;

enum Instruction {
    Pick(Oid),
    Push(Oid, String),
}

impl Execute for Push {
    fn execute(&self) -> Result<(), ()> {
        let git = Git::open(".");

        let commits = git.list_commits("main");
        let output = commits_to_string(commits, UiFilter::OnlyTargets);

        let file_path = "/tmp/yggit";

        let output = format!("{}\n{}", output, COMMENTS);
        std::fs::write(file_path, output).map_err(|_| println!("cannot write file to disk"))?;

        let content = git.edit_file(file_path)?;

        let commits = instruction_from_string(content).ok_or_else(|| {
            println!("Cannot parse instructions");
        })?;

        let commits = merge_notes(&git, commits, NoteMergingPolicy::OnlyTarget);

        let instruction = commits
            .iter()
            .fold(Vec::default(), |mut acc, commit| {
                let oid = commit.id;
                let note = &commit.note;
                acc.push(Instruction::Pick(oid));
                if let Some(Note {
                    push: Some(crate::core::Push { target }),
                    ..
                }) = note
                {
                    acc.push(Instruction::Push(oid, target.clone()));
                }
                acc
            })
            .iter()
            .map(|instruction| match instruction {
                Instruction::Pick(oid) => format!("pick {oid}"),
                Instruction::Push(oid, target) => {
                    format!("exec git push origin {oid}:refs/heads/{target} --force-with-lease")
                }
            })
            .collect::<Vec<String>>()
            .join("\n");

        git.start_rebase("main");
        git.write_todo(&instruction);
        // git.rebase_continue();

        println!("DONE");

        Ok(())
    }
}
