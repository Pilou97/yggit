use git2::Repository;
use std::env;
use thiserror::Error;

pub trait Config {
    /// Returns the name of the signer
    fn name(&self) -> &String;

    /// Returns the email of the signer
    fn email(&self) -> &String;

    /// Returns the editor of the signer
    fn editor(&self) -> &String;
}

pub struct GitConfig {
    name: String,
    email: String,
    editor: String,
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Cannot read the git config")]
    CannotReadConfig,
    #[error("value {0} is not present in git config")]
    ValueNotPresentInConfig(&'static str),
    #[error("env variable {0} is not set")]
    ValueNotPresentInEnv(&'static str),
    #[error("notes.rewriteRef has to be set to \"refs/notes/commits\"")]
    WrongRewriteRefValue,
}

impl GitConfig {
    pub fn new(repository: &Repository) -> Result<Self, ConfigError> {
        let config = repository
            .config()
            .map_err(|_| ConfigError::CannotReadConfig)?;

        let name = config
            .get_string("user.name")
            .map_err(|_| ConfigError::ValueNotPresentInConfig("user.name"))?;

        let email = config
            .get_string("user.email")
            .map_err(|_| ConfigError::ValueNotPresentInConfig("user.email"))?;

        let editor = config.get_string("core.editor").or_else(|_| {
            env::var("EDITOR").map_err(|_| ConfigError::ValueNotPresentInEnv("EDITOR"))
        })?;

        // Force rewriteRef = "refs/notes/commits" to exist
        let rewrite_ref = config
            .get_string("notes.rewriteRef")
            .map_err(|_| ConfigError::ValueNotPresentInConfig("notes.rewriteRef"))?;
        if rewrite_ref != "refs/notes/commits" {
            return Err(ConfigError::WrongRewriteRefValue);
        }

        Ok(GitConfig {
            name,
            email,
            editor,
        })
    }
}

impl Config for GitConfig {
    fn name(&self) -> &String {
        &self.name
    }

    fn email(&self) -> &String {
        &self.email
    }

    fn editor(&self) -> &String {
        &self.editor
    }
}
