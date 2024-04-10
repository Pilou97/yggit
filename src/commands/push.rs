use crate::{
    core::{apply, push_from_notes, save_note},
    git::{Editor, Git},
    parser::{commits_to_string, instruction_from_string},
};
use anyhow::{Context, Result};
use clap::Args;

#[derive(Debug, Args)]
pub struct Push {
    /// use --force to update branches,
    /// by default it is using --force-with-lease
    #[arg(short, long, default_value_t = false)]
    force: bool,
}

const COMMENTS: &str = r#"
# Here is how to use yggit
# 
# Commands:
# -> <branch> add a branch to the above commit
# -> <origin>:<branch> add a branch to the above commit
# 
# What happens next?
#  - All branches are pushed on origin, except if you specified a custom origin
#
# It's not a rebase, you can't edit commits nor reorder them
"#;

impl Push {
    pub fn execute(&self, git: Git<impl Editor>) -> Result<()> {
        let commits = git.list_commits()?;
        let output = commits_to_string(commits);
        let output = format!("{}\n{}", output, COMMENTS);

        let content = git.edit_text(output)?;

        let commits = instruction_from_string(content).context("Cannot parse instruction")?;

        save_note(&git, commits)?;
        apply(&git)?;
        push_from_notes(&git, self.force)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        git::Git,
        tests::{git_cmd::GitCmd, mocked_ui::MockedUi},
    };
    use git2::Oid;

    use super::Push;

    /// helper that initialize a repository with one commit
    ///
    /// It returns the head and the repository
    fn init_repo_with_commit() -> (Oid, GitCmd) {
        let repo = GitCmd::init_bare("main");
        repo.new_file(
            "readme.md",
            concat!("# Star wars", "\n", "Hello there\n", "General Kenobi\n"),
        );
        repo.add_all();
        let oid = repo.commit("first commit");
        repo.add_note(oid, &"my super note".to_string());
        (oid, repo)
    }

    #[test]
    fn push_new_branch() {
        let (_, git_cmd) = init_repo_with_commit();
        git_cmd.create_branch("pilou@osecour");
        git_cmd.new_file("test.md", "hello there");
        git_cmd.add_all();
        let _ = git_cmd.commit("test.md");

        let mut git = Git::<MockedUi>::open(&git_cmd.path()).unwrap();

        git.editor.set_editor(|string| {
            let mut splitted = string.split("\n").collect::<Vec<&str>>();
            splitted.insert(1, "-> my-new-branch");
            let commits = splitted.join("\n");

            println!("{}", commits);
            Ok(commits)
        });

        let cmd = Push { force: false };
        cmd.execute(git).unwrap();

        // origin/my-new-branch should be the same as HEAD
        let head = git_cmd.get_commit_of_branch("HEAD");
        let origin = git_cmd.get_commit_of_branch("origin/my-new-branch");
        let local = git_cmd.get_commit_of_branch("my-new-branch");

        assert_eq!(head, origin);
        assert_eq!(origin, local);
        assert_eq!(local, head);
    }
}
