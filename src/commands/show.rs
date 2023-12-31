use clap::Args;

use crate::{git::Git, parser::commits_to_string};

use super::Execute;

#[derive(Debug, Args)]
pub struct Show {}

const COMMENTS: &str = r#"
# Only display the state of the branches
"#;

impl Execute for Show {
    fn execute(&self) -> Result<(), ()> {
        let git = Git::open(".");
        let commits = git.list_commits();
        let output = commits_to_string(commits);

        let file_path = "/tmp/yggit";
        let output = format!("{}\n{}", output, COMMENTS);
        std::fs::write(file_path, output).map_err(|_| println!("cannot write file to disk"))?;

        git.edit_file(file_path)?;

        Ok(())
    }
}
