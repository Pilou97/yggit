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
    /// Load a .gitconfig from the current directory
    ///
    /// If the .gitconfig is not found, the function will try to load the gitconfig from the parent directory
    /// until there is no more parent
    pub fn open() -> Result<GitConfig, ()> {
        let config = git2::Config::open_default().map_err(|_| ())?;

        let email = config
            .get_string("user.email")
            .map_err(|_| println!("email not found in configuration"))?;

        let name = config
            .get_string("user.name")
            .map_err(|_| println!("name not found in configuration"))?;

        let editor = (match config.get_string("core.editor") {
            Ok(editor) => Ok(editor),
            Err(_) => {
                std::env::var("EDITOR").map_err(|_| println!("editor not found in configuration"))
            }
        })?;

        // Force rewriteRef = "refs/notes/commits" to exist
        let rewrite_ref = config
            .get_string("notes.rewriteRef")
            .map_err(|_| println!("editor not found in configuration"))?;
        if rewrite_ref != "refs/notes/commits" {
            println!("rewriteRef should be set to \"refs/notes/commits\"");
            return Err(());
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
