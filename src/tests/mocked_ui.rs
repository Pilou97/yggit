use crate::git::{Editor, GitConfig};
use anyhow::Result;

pub struct MockedUi {
    editor: Option<fn(String) -> Result<String>>,
}

impl Editor for MockedUi {
    fn new(_config: &GitConfig) -> anyhow::Result<Self> {
        Ok(MockedUi { editor: None })
    }

    fn edit(&self, content: String) -> Result<String> {
        let Some(editor) = self.editor else {
            return Err(anyhow::Error::msg("editor not set".to_string()));
        };
        editor(content)
    }
}

impl MockedUi {
    /// Inject the editor
    ///
    /// The editor is function called by "edit_content" in git.rs
    pub fn set_editor(&mut self, editor: fn(String) -> Result<String>) {
        self.editor = Some(editor)
    }
}
