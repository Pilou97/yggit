#[derive(Debug)]
pub struct GitConfig {
    pub user: User,
    pub core: Core,
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

        let editor = config
            .get_string("core.editor")
            .map_err(|_| println!("editor not found in configuration"))?;

        Ok(Self {
            user: User { email, name },
            core: Core { editor },
        })
    }
}
