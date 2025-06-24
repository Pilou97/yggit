use git2::{Oid, Repository, Signature};
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use thiserror::Error;

/// Simple key value store
///
/// The values are stored in the commit note
/// Don't forget to set rewriteRef to "refs/notes/commits"
pub struct GitDatabase<'a> {
    repository: &'a Repository,
    name: String,
    email: String,
}

#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("Unknown error")]
    Unknown,
    #[error("Cannot serialize value")]
    CannotSerializeValue,
    #[error("Cannot deserialize value")]
    CannotDeserializeValue,
    #[error("Cannot serialize database")]
    CannotSerialize,
    #[error("Cannot open database")]
    CannotOpen,
    #[error("Cannot close database")]
    CannotClose,
}

pub trait DatabaseRead {
    /// Retrieve data from the commit note
    fn read<D>(&self, oid: &Oid, key: &str) -> Result<Option<D>, DatabaseError>
    where
        D: DeserializeOwned;
}

pub trait DatabaseWrite {
    /// Stores data in the commit note
    fn write<D>(&self, oid: &Oid, key: &str, data: &D) -> Result<(), DatabaseError>
    where
        D: Serialize;
}

pub trait DatabaseDelete {
    /// Delete the key for a given note
    fn delete(&self, oid: &Oid, key: &str) -> Result<(), DatabaseError>;
}

pub trait Database: DatabaseRead + DatabaseDelete + DatabaseWrite {}

impl<'a> GitDatabase<'a> {
    pub fn new(repository: &'a Repository, name: String, email: String) -> Self {
        GitDatabase {
            repository,
            name,
            email,
        }
    }

    /// Read the notes stored for the given Oid
    fn read_note(&self, oid: &Oid) -> HashMap<String, Value> {
        self.repository
            .find_note(None, *oid)
            .map(|note| {
                let message = note.message().unwrap_or_default();
                serde_json::from_str::<HashMap<String, Value>>(message).unwrap_or_default()
            })
            .unwrap_or_default()
    }

    /// Write the note and erase the old one
    fn write_note(&self, oid: &Oid, note: HashMap<String, Value>) -> Result<(), DatabaseError> {
        let note = serde_json::to_string(&note).map_err(|_| DatabaseError::CannotSerialize)?;
        let author = Signature::now(&self.name, &self.email).unwrap();
        self.repository
            .note(&author, &author, None, *oid, &note, true)
            .map(|_| ())
            .map_err(|_| DatabaseError::CannotClose)
    }
}

impl DatabaseWrite for GitDatabase<'_> {
    fn write<D>(&self, oid: &Oid, key: &str, data: &D) -> Result<(), DatabaseError>
    where
        D: Serialize,
    {
        let mut note = self.read_note(oid);

        let data = serde_json::to_value(data).map_err(|_| DatabaseError::CannotSerializeValue)?;
        note.insert(key.to_string(), data);

        self.write_note(oid, note)
    }
}
impl DatabaseRead for GitDatabase<'_> {
    fn read<D>(&self, oid: &Oid, key: &str) -> Result<Option<D>, DatabaseError>
    where
        D: DeserializeOwned,
    {
        let note = self.read_note(oid);

        let Some(value) = note.get(key) else {
            return Ok(None);
        };
        serde_json::from_value::<D>(value.clone())
            .map(Some)
            .map_err(|_| DatabaseError::CannotDeserializeValue)
    }
}

impl DatabaseDelete for GitDatabase<'_> {
    fn delete(&self, oid: &Oid, key: &str) -> Result<(), DatabaseError> {
        let mut note = self.read_note(oid);
        note.remove(key);
        self.write_note(oid, note)
    }
}

impl Database for GitDatabase<'_> {}

#[cfg(test)]
mod tests {
    use crate::{DatabaseDelete, DatabaseRead, DatabaseWrite, GitDatabase};
    use yggit_test::TempRepository;

    #[test]
    fn test_get_note() {
        // Init the repository
        let repository = TempRepository::new();
        repository.set_identity("Bob", "example@example.com");
        repository.add_file("README.md", "a cool readme");
        repository.commit("a commit message");
        let repo = repository.as_ref();
        // Get the head commit
        let id = repo.head().unwrap().peel_to_commit().unwrap().id();

        // Test the db
        let database = GitDatabase::new(&repo, "My name".into(), "My email".into());

        assert!(database.read::<String>(&id, "hello").unwrap().is_none());
        assert!(database.write(&id, "hello", &"data".to_string()).is_ok());
        assert_eq!(
            "data",
            database.read::<String>(&id, "hello").unwrap().unwrap()
        );
        database.delete(&id, &"hello").expect("should work");
        assert!(database.read::<String>(&id, "hello").unwrap().is_none());
    }
}
