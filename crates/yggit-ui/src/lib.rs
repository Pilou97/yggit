use std::process::Command;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum EditorError {
    #[error("Cannot edit todo's list because [{0}]")]
    CannotEdit(&'static str),
}

pub trait Editor {
    fn edit(&self, content: String) -> Result<String, EditorError>;
}

pub struct GitEditor {
    editor: String,
}

impl GitEditor {
    pub fn new(editor: String) -> GitEditor {
        GitEditor { editor }
    }
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

impl Editor for GitEditor {
    fn edit(&self, content: String) -> Result<String, EditorError> {
        // We need to create a file
        let file_path = "/tmp/yggit";
        let output = format!("{}\n{}", content, COMMENTS);
        std::fs::write(file_path, output)
            .map_err(|_| EditorError::CannotEdit("cannot initiate todo's list"))?;

        let output = Command::new(&self.editor)
            .arg(file_path)
            .status()
            .map_err(|_| EditorError::CannotEdit("cannot open todo in editor"))?;

        let true = output.success() else {
            return Err(EditorError::CannotEdit("editor did not correctly end"));
        };
        let content = std::fs::read_to_string(file_path)
            .map_err(|_| EditorError::CannotEdit("cannot read result from editor"))?;
        Ok(content)
    }
}
