use anyhow::{Context, Result};
use std::process::Command;

use super::config::GitConfig;

/// Trait that defined the UI of the user
pub trait Editor: Sized {
    /// Create an instance of the editor
    fn new(config: &GitConfig) -> Result<Self>;

    /// Edit the given content
    fn edit(&self, content: String) -> Result<String>;
}

/// Terminal editor
///
/// It will open the editor specified in the git configuration
pub struct Terminal {
    command: String, // the command to open the editor, like "neovim"
}

impl Editor for Terminal {
    fn new(config: &GitConfig) -> Result<Self> {
        Ok(Terminal {
            command: config.core.editor.clone(),
        })
    }

    fn edit(&self, content: String) -> Result<String> {
        let file_path = "/tmp/yggit";
        // Write the content to the file
        std::fs::write(file_path, content).context("cannot write file to disk")?;
        // Open the editor
        let output = Command::new(&self.command)
            .arg(file_path)
            .status()
            .context("Failed to open editor")?;
        let true = output.success() else {
            return Err(anyhow::Error::msg("Editor did not end successfully"));
        };
        // Read the content of the file
        let content =
            std::fs::read_to_string(file_path).context("Cannot read string from editor")?;
        Ok(content)
    }
}
