use std::{fs::File, io::Read, path::Path};

use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct GitConfig {
    pub user: User,
    pub yggit: Yggit,
    pub core: Core,
}

#[derive(Deserialize, Debug)]
pub struct User {
    pub email: String,
    pub name: String,
}

#[derive(Deserialize, Debug)]
pub struct Yggit {
    #[serde(rename = "privateKey")]
    pub private_key: String,
}

#[derive(Deserialize, Debug)]
pub struct Core {
    pub editor: String,
}

impl GitConfig {
    /// Load a .gitconfig from the current directory
    ///
    /// If the .gitconfig is not found, the function will try to load the gitconfig from the parent directory
    /// until there is no more parent
    pub fn from_directory(path: &Path) -> Result<GitConfig, ()> {
        let file = path.join(".gitconfig");
        println!("{:?}", file);
        let file = File::open(file);
        match file {
            Ok(mut file) => {
                let mut contents = String::new();
                file.read_to_string(&mut contents)
                    .expect("Failed to read the file.");
                let git_config: GitConfig = toml::from_str(&contents).map_err(|_| ())?;
                Ok(git_config)
            }
            Err(_) => {
                let path = path.parent().ok_or(())?;
                Self::from_directory(path)
            }
        }
    }
}
