use anyhow::{Context, Result};

#[derive(Debug)]
pub struct GitConfig {
    pub user: User,
    pub core: Core,
    pub yggit: Yggit,
}

#[derive(Debug)]
pub struct User {
    pub email: String,
    pub name: String,
}

#[derive(Debug)]
pub struct Core {
    pub editor: String,
}

#[derive(Debug)]
pub struct Yggit {
    // Default upstream of a branch
    pub default_upstream: String,
}

impl GitConfig {
    /// Try to load gitconfig from:
    ///  - global: $HOME/.gitconfig
    ///  - XDG: $HOME/.config/git/config
    ///  - system: /etc/gitconfig
    pub fn open_default() -> Result<GitConfig> {
        let config = git2::Config::open_default().context("Cannot open git config")?;
        Self::open_with_git_config(config)
    }

    /// Parse the git config and return a Config
    ///
    /// It parses the following field:
    ///  - user.email : required
    ///  - user.name : required
    ///  - notes.rewriteRef = "refs/notes/commits" : required
    ///  - yggit.defaultUpstream : optional, default(origin)
    fn open_with_git_config(config: git2::Config) -> Result<GitConfig> {
        let email = config
            .get_string("user.email")
            .context("email not found in configuration")?;

        let name = config
            .get_string("user.name")
            .context("name not found in configuration")?;

        let editor = (match config.get_string("core.editor") {
            Ok(editor) => Ok(editor),
            Err(_) => std::env::var("EDITOR").context("editor not found in configuration"),
        })?;

        // Force rewriteRef = "refs/notes/commits" to exist
        let rewrite_ref = config
            .get_string("notes.rewriteRef")
            .context("notes.rewriteRef wasn't found")?;
        if rewrite_ref != "refs/notes/commits" {
            println!("rewriteRef should be set to \"refs/notes/commits\"");
            return Err(anyhow::Error::msg(
                "rewriteRef should be set to \"refs/notes/commits\"",
            ));
        }

        let default_upstream = config
            .get_string("yggit.defaultUpstream")
            .unwrap_or("origin".to_string());

        Ok(Self {
            user: User { email, name },
            core: Core { editor },
            yggit: Yggit { default_upstream },
        })
    }
}
