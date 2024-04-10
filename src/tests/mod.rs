/// This module provides many helpful types for test purpose
/// All the file should only be compiled with test flag

/// GitCmd is a wrapper around the git client
/// It's mainly used to initiate repository without yggit, or to create commit
/// This lib is critic because all the tests are based on it
#[cfg(test)]
pub mod git_cmd;

/// The editor of yggit is abstracted and injected correctly to Git
/// Of course I can't test every editor
/// So to test the logic of the commands, we can replace the editor by a MockedUi
/// This editor allows to replace what an editor does
#[cfg(test)]
pub mod mocked_ui;
