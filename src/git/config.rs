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

#[cfg(test)]
mod tests {
    use super::GitConfig;
    use anyhow::{Context, Result};
    use std::{fs::File, io::Write, path::Path};
    use tempfile::TempDir;

    impl GitConfig {
        fn open(path: &Path) -> Result<GitConfig> {
            let config = git2::Config::open(path).context("config not found")?;
            Self::open_with_git_config(config)
        }
    }

    #[test]
    fn test_open() {
        let tmp_dir = TempDir::new().expect("should be created");
        let config = concat!(
            "[user]\n",
            "email = kenobi@example.com\n",
            "name = Obi-Wan\n",
            "[core]\n",
            "editor = neovim\n",
            "[notes]\n",
            "rewriteRef = refs/notes/commits\n",
            "[yggit]\n",
            "defaultUpstream = origin\n"
        );

        let path = tmp_dir.path().join(".gitconfig");
        let mut file = File::create(&path).expect("gitconfig should be created");
        file.write(config.as_bytes()).expect("should be written");

        let config = GitConfig::open(&path).expect("should be open");
        assert_eq!(config.user.email, "kenobi@example.com");
        assert_eq!(config.user.name, "Obi-Wan");
        assert_eq!(config.core.editor, "neovim");
        assert_eq!(config.yggit.default_upstream, "origin");
    }

    #[test]
    fn test_open_missing_email() {
        let tmp_dir = TempDir::new().expect("should be created");
        let config = concat!(
            "[user]\n",
            "name = Obi-Wan\n",
            "[core]\n",
            "editor = neovim\n",
            "[notes]\n",
            "rewriteRef = refs/notes/commits\n",
            "[yggit]\n",
            "defaultUpstream = origin\n"
        );

        let path = tmp_dir.path().join(".gitconfig");
        let mut file = File::create(&path).expect("gitconfig should be created");
        file.write(config.as_bytes()).expect("should be written");

        let config = GitConfig::open(&path);
        assert!(config.is_err());
        assert_eq!(
            config.unwrap_err().to_string(),
            "email not found in configuration"
        )
    }

    #[test]
    fn test_open_missing_name() {
        let tmp_dir = TempDir::new().expect("should be created");
        let config = concat!(
            "[user]\n",
            "email = kenobi@example.com\n",
            "[core]\n",
            "editor = neovim\n",
            "[notes]\n",
            "rewriteRef = refs/notes/commits\n",
            "[yggit]\n",
            "defaultUpstream = origin\n"
        );

        let path = tmp_dir.path().join(".gitconfig");
        let mut file = File::create(&path).expect("gitconfig should be created");
        file.write(config.as_bytes()).expect("should be written");

        let config = GitConfig::open(&path);
        assert!(config.is_err());
        assert_eq!(
            config.unwrap_err().to_string(),
            "name not found in configuration"
        )
    }

    #[test]
    fn test_open_missing_editor() {
        let tmp_dir = TempDir::new().expect("should be created");
        std::env::remove_var("EDITOR");

        let config = concat!(
            "[user]\n",
            "email = kenobi@example.com\n",
            "name = Obi-Wan\n",
            "[notes]\n",
            "rewriteRef = refs/notes/commits\n",
            "[yggit]\n",
            "defaultUpstream = origin\n"
        );

        let path = tmp_dir.path().join(".gitconfig");
        let mut file = File::create(&path).expect("gitconfig should be created");
        file.write(config.as_bytes()).expect("should be written");

        let config = GitConfig::open(&path);
        assert!(config.is_err());
        assert_eq!(
            config.unwrap_err().to_string(),
            "editor not found in configuration"
        );

        //  Other test that set the EDITOR var
        let tmp_dir = TempDir::new().expect("should be created");
        std::env::set_var("EDITOR", "emacs");

        let config = concat!(
            "[user]\n",
            "email = kenobi@example.com\n",
            "name = Obi-Wan\n",
            "[notes]\n",
            "rewriteRef = refs/notes/commits\n",
            "[yggit]\n",
            "defaultUpstream = origin\n"
        );

        let path = tmp_dir.path().join(".gitconfig");
        let mut file = File::create(&path).expect("gitconfig should be created");
        file.write(config.as_bytes()).expect("should be written");

        let config = GitConfig::open(&path).expect("should be ok");
        assert_eq!(config.core.editor, "emacs");
    }

    #[test]
    fn test_open_missing_rewrite_ref() {
        let tmp_dir = TempDir::new().expect("should be created");

        let config = concat!(
            "[user]\n",
            "email = kenobi@example.com\n",
            "name = Obi-Wan\n",
            "[core]\n",
            "editor = neovim\n",
            "[yggit]\n",
            "defaultUpstream = origin\n"
        );

        let path = tmp_dir.path().join(".gitconfig");
        let mut file = File::create(&path).expect("gitconfig should be created");
        file.write(config.as_bytes()).expect("should be written");

        let config = GitConfig::open(&path);
        assert!(config.is_err());
        assert_eq!(
            config.unwrap_err().to_string(),
            "notes.rewriteRef wasn't found"
        )
    }

    #[test]
    fn test_open_wrong_rewrite_ref() {
        let tmp_dir = TempDir::new().expect("should be created");

        let config = concat!(
            "[user]\n",
            "email = kenobi@example.com\n",
            "name = Obi-Wan\n",
            "[core]\n",
            "editor = neovim\n",
            "[notes]\n",
            "rewriteRef = wrong-value\n",
            "[yggit]\n",
            "defaultUpstream = origin\n"
        );

        let path = tmp_dir.path().join(".gitconfig");
        let mut file = File::create(&path).expect("gitconfig should be created");
        file.write(config.as_bytes()).expect("should be written");

        let config = GitConfig::open(&path);
        assert!(config.is_err());
        assert_eq!(
            config.unwrap_err().to_string(),
            "rewriteRef should be set to \"refs/notes/commits\""
        )
    }

    #[test]
    fn test_default_upstream() {
        let tmp_dir = TempDir::new().expect("should be created");
        let config = concat!(
            "[user]\n",
            "email = kenobi@example.com\n",
            "name = Obi-Wan\n",
            "[core]\n",
            "editor = neovim\n",
            "[notes]\n",
            "rewriteRef = refs/notes/commits\n",
        );

        let path = tmp_dir.path().join(".gitconfig");
        let mut file = File::create(&path).expect("gitconfig should be created");
        file.write(config.as_bytes()).expect("should be written");

        let config = GitConfig::open(&path).expect("should be open");
        assert_eq!(config.yggit.default_upstream, "origin");
    }

    #[test]
    fn test_default_upstream_with_different_value() {
        let tmp_dir = TempDir::new().expect("should be created");
        let config = concat!(
            "[user]\n",
            "email = kenobi@example.com\n",
            "name = Obi-Wan\n",
            "[core]\n",
            "editor = neovim\n",
            "[notes]\n",
            "rewriteRef = refs/notes/commits\n",
            "[yggit]\n",
            "defaultUpstream = upstream"
        );

        let path = tmp_dir.path().join(".gitconfig");
        let mut file = File::create(&path).expect("gitconfig should be created");
        file.write(config.as_bytes()).expect("should be written");

        let config = GitConfig::open(&path).expect("should be open");
        assert_eq!(config.yggit.default_upstream, "upstream");
    }
}
