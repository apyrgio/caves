//! # Caves
//!
//! A selection of key-value stores (kvs) with the following features:
//!
//! * [Embedded]
//! * Thread-safe
//! * Simple API; get/set/delete a key
//! * Dev-friendly
//!
//! The latter is the main reason for creating this crate. By dev-friendly we
//! mean that all of the key-values stores provide the same interface with the
//! same semantics. Therefore, the developer can use each kv interchangeably,
//! according to their needs.
//!
//! The only differences that the developer needs to know for each kv are:
//!
//! * Naming restrictions: Some kvs may have restrictions regarding the
//!   characters in a name. For instance, the file kv does not allow the `/`
//!   character.
//! * Persistence guarantees: The kvs do not offer the same guarantees once
//!   the plug is pulled. For instance, the memory kv does not retain state
//!   when the power is lost.
//!
//! The uniformity in the interface of the kvs is enforced by the [`Cave`]
//! trait. See its definition for more info.
//!
//! [Embedded]: https://en.wikipedia.org/wiki/Embedded_database
//! [`Cave`]: trait.Cave.html

#![deny(
    warnings,
    missing_copy_implementations,
    missing_debug_implementations,
    missing_docs,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unstable_features,
    unused_import_braces,
    unused_qualifications,
    unused_extern_crates,
    unused_must_use,
    unused_results,
    variant_size_differences
)]

#[macro_use]
extern crate anyhow;

pub mod errors;
pub mod res;

use std::collections;
use std::fs;
use std::io;
use std::io::Write;
use std::path;
use std::sync;

use atomicwrites;

use crate::errors::Error;
use crate::res::{empty_ok, Res};

/// A simple interface for key-value stores.
///
/// A `Cave` object must have support for the following actions:
///
/// * Get a key by name, or return an error if it doesn't exist.
/// * Store a key by name; update it if it exists or create it if it doesn't.
/// * Delete a key by name, or return an error if it doesn't exist.
///
/// These actions must be able to happen concurrently, from any thread. This
/// means that the objects must not rely on exclusive mutability references in
/// order to update their state.
///
/// ## Usage
///
/// Here's an example on how one can use a `Cave` object. In this example, we
/// use a `MemoryCave` object, but any other `Cave` object can be used in its
/// place.
///
/// ```
/// use caves::errors::Error;
/// use caves::{MemoryCave, Cave};
///
/// // Initialize a MemoryCave object.
/// let b = MemoryCave::new();
///
/// // Create a new key with an empty value.
/// b.set("key", b"");
///
/// // Override the key's value.
/// b.set("key", b"value");
///
/// // Retrieve the contents of the key.
/// let res = b.get("key");
/// assert_eq!(res.unwrap(), b"value");
///
/// // Delete the key.
/// b.delete("key");
///
/// // Subsequent attempts to retrieve the contents of the key should return an
/// // error.
/// let res = b.get("key");
/// assert_eq!(res, Err(Error::NotFound("key".to_string())));
/// ```
pub trait Cave: Send + Sync {
    /// Get a key by its name, and return its contents.
    ///
    /// If it does not exist, return an error.
    fn get(&self, name: &str) -> Res;

    /// Create or update a key by its name.
    fn set(&self, name: &str, data: &[u8]) -> Res;

    /// Delete a key by its name.
    ///
    /// If it does not exist, return an error.
    fn delete(&self, name: &str) -> Res;

    /// A helper method to return an error for keys that could not be found.
    fn not_found(&self, name: &str) -> Res {
        Err(Error::NotFound(name.into()))
    }
}

/// A key-value store that stores keys in-memory.
///
/// This kv uses an in-memory hash table to store keys and their contents.
///
/// ## Caveats
///
/// This kv has the following caveats:
///
/// * Since it uses an in-memory hash table, in case of a power-cycle, all data
///   will be lost.
/// * In order to make the hash table thread-safe, we protect it with a
///   read-write lock. This makes it prohibitive for write-intensive workloads.
///
/// Consider using this kv for testing purposes or short-lived installations,
/// but avoid it for any other scenario.
#[derive(Debug)]
pub struct MemoryCave {
    hash_map: sync::RwLock<collections::HashMap<String, Vec<u8>>>,
}

impl MemoryCave {
    /// Create a new instance.
    pub fn new() -> Self {
        Self {
            hash_map: sync::RwLock::new(collections::HashMap::new()),
        }
    }
}

impl Cave for MemoryCave {
    fn get(&self, name: &str) -> Res {
        match self.hash_map.read().unwrap().get(name) {
            Some(data) => Ok(data.to_vec()),
            None => self.not_found(name),
        }
    }

    fn set(&self, name: &str, data: &[u8]) -> Res {
        let _ = self
            .hash_map
            .write()
            .unwrap()
            .insert(name.to_string(), data.to_vec());
        empty_ok()
    }

    fn delete(&self, name: &str) -> Res {
        match self.hash_map.write().unwrap().remove(name) {
            Some(_) => empty_ok(),
            None => self.not_found(name),
        }
    }
}

/// A key-value store that stores keys in files.
///
/// This kv stores keys as files in a directory. Note that the directory must
/// exist.
///
/// ## Caveats
///
/// This kv has the following caveats:
///
/// * It doesn't perform a sync operation after every set/delete.
/// * It doesn't create multi-level directories, e.g., `fi/le/name`, to improve
///   filesystem lookups.
///
/// Consider using it when you want to audit which keys are created using
/// external tools, such as `ls`, `cat`.
#[derive(Debug)]
pub struct FileCave {
    dir: path::PathBuf,
}

impl FileCave {
    /// Create a new instance.
    ///
    /// Check if the provided path is a directory and that it exists.
    pub fn new(dir: &path::Path) -> Result<Self, Error> {
        // Return an error if the path is invalid or if we don't have enough
        // permissions to get its metadata [1].
        //
        // [1]: https://doc.rust-lang.org/std/fs/fn.metadata.html#errors
        let md = match fs::metadata(dir) {
            Err(e) => return Err(Error::Internal(e.into())),
            Ok(md) => md,
        };

        // Return an error if the path is valid, but is not a directory.
        if !md.is_dir() {
            return Err(Error::internal_from_msg(format!(
                "Provided path is not a directory: {:?}",
                dir
            )));
        }

        Ok(Self {
            dir: dir.to_owned(),
        })
    }

    fn create_path(&self, name: &str) -> path::PathBuf {
        self.dir.join(name)
    }

    fn convert_io_error(e: io::Error, name: &str) -> Error {
        match e.kind() {
            io::ErrorKind::NotFound => Error::NotFound(name.into()),
            _ => Error::Internal(e.into()),
        }
    }
}

impl Cave for FileCave {
    fn get(&self, name: &str) -> Res {
        let path = self.create_path(name);

        match fs::read(path) {
            Ok(buf) => Ok(buf),
            Err(e) => Err(Self::convert_io_error(e, name)),
        }
    }

    fn set(&self, name: &str, data: &[u8]) -> Res {
        let path = self.create_path(name);

        let af = atomicwrites::AtomicFile::new(path, atomicwrites::AllowOverwrite);
        match af.write(|f| f.write_all(data)) {
            Ok(_) => empty_ok(),
            // The `atomicwrites` crate provides two types of errors [1]:
            //
            // * Internal: This is a library error that happens when the
            //   tempfile cannot be created or moved. We treat it as an
            //   internal error, because it's essentially an io:Error that can
            //   happen, e.g., if there are no proper permissions in the
            //   directory.
            // * User: This the error of the lambda expression. In our case,
            //   our lambda is very simple so we can't have a bug. If it fails,
            //   it may be due to a ENOSPC error, which is also an internal
            //   error.
            //
            // So, that's why we treat all the `atomicwrites` errors as
            // internal errors.
            //
            // [1]: https://docs.rs/atomicwrites/0.2.5/atomicwrites/enum.Error.html
            Err(e) => Err(Error::Internal(e.into())),
        }
    }

    fn delete(&self, name: &str) -> Res {
        let path = self.create_path(name);
        match fs::remove_file(path) {
            Ok(_) => empty_ok(),
            Err(e) => Err(Self::convert_io_error(e, name)),
        }
    }
}

/// A key-value store that stores keys in [RocksDB].
///
/// [RocksDB]: https://github.com/facebook/rocksdb
#[cfg(feature = "with-rocksdb")]
#[derive(Debug)]
pub struct RocksDBCave {
    db: rocksdb::DB,
}

#[cfg(feature = "with-rocksdb")]
impl RocksDBCave {
    /// Create a new instance.
    ///
    /// If the provided directory does not exist, it will be created.
    pub fn new(dir: &path::Path) -> Result<Self, Error> {
        match rocksdb::DB::open_default(dir) {
            Ok(db) => Ok(Self { db }),
            Err(e) => Err(Error::Internal(e.into())),
        }
    }
}

#[cfg(feature = "with-rocksdb")]
impl Cave for RocksDBCave {
    fn get(&self, name: &str) -> Res {
        match self.db.get(name.as_bytes()) {
            Ok(o) => match o {
                Some(buf) => Ok(buf),
                None => self.not_found(name),
            },
            Err(e) => Err(Error::Internal(e.into())),
        }
    }

    fn set(&self, name: &str, data: &[u8]) -> Res {
        match self.db.put(name.as_bytes(), data) {
            Ok(_) => empty_ok(),
            Err(e) => Err(Error::Internal(e.into())),
        }
    }

    fn delete(&self, name: &str) -> Res {
        // XXX: We should find a better way to check if a value exists or not.
        match self.get(name) {
            Ok(_) => (),
            e => return e,
        }

        match self.db.delete(name.as_bytes()) {
            Ok(_) => empty_ok(),
            Err(e) => Err(Error::Internal(e.into())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use assert_fs;

    fn _test_simple(b: Box<dyn Cave>) {
        let not_found_err = Err(Error::NotFound("test".to_string()));
        let value1 = Ok("value".as_bytes().to_vec());
        let value2 = Ok("value2".as_bytes().to_vec());
        let value3 = Ok("value3".as_bytes().to_vec());

        let res = b.get("test");
        assert_eq!(res, not_found_err);
        let res = b.delete("test");
        assert_eq!(res, not_found_err);
        let res = b.set("test", "value".as_bytes());
        assert_eq!(res, empty_ok());
        let res = b.get("test");
        assert_eq!(res, value1);
        let res = b.set("test", "value2".as_bytes());
        assert_eq!(res, empty_ok());
        let res = b.get("test");
        assert_eq!(res, value2);
        let res = b.delete("test");
        assert_eq!(res, empty_ok());
        let res = b.get("test");
        assert_eq!(res, not_found_err);
        let res = b.delete("test");
        assert_eq!(res, not_found_err);
        let res = b.set("test", "value3".as_bytes());
        assert_eq!(res, empty_ok());
        let res = b.get("test");
        assert_eq!(res, value3);
    }

    #[test]
    fn test_memory_backend_simple() {
        let mb = MemoryCave::new();
        _test_simple(Box::new(mb))
    }

    #[test]
    fn test_file_backend_simple() {
        let temp_dir = assert_fs::TempDir::new().unwrap();
        let fb = FileCave::new(temp_dir.path()).unwrap();
        _test_simple(Box::new(fb))
    }

    #[test]
    fn test_file_backend_errors() {
        let temp_dir = assert_fs::TempDir::new().unwrap();
        let internal_err = Error::Internal(anyhow!(""));

        // Test for non-existent paths.
        let no_path = temp_dir.path().join("nonexistent");
        let res = FileCave::new(&no_path);
        assert_eq!(res.is_err(), true);
        let err = res.unwrap_err();
        assert_eq!(err, internal_err);
        // XXX: In order to see if the error is ENOENT, we have to somehow get
        // it from `anyhow`. We can't check the string representation of the
        // error, because it's different betweeen Windows and Linux/MacOs hosts.
        //let msg = format!("{:?}", err);
        //assert_eq!(msg.contains("No such file or directory"), true);

        // Test for files instead of directories.
        let empty_file = temp_dir.path().join("empty_file");
        let res = fs::File::create(&empty_file);
        assert_eq!(res.is_ok(), true);
        let res = FileCave::new(&empty_file);
        assert_eq!(res.is_err(), true);
        let err = res.unwrap_err();
        assert_eq!(err, internal_err);
        // XXX: We can't check the string representation of the error. See
        // previous similar comment.
        //let msg = format!("{:?}", err);
        //assert_eq!(msg.contains("is not a directory"), true);

        // Test for removed directory under our feet.
        let internal_err = Err(internal_err);
        let not_found_err: Res = Err(Error::NotFound("test".to_string()));
        let dir = temp_dir.path().join("dir");
        let res = fs::create_dir(&dir);
        assert_eq!(res.is_ok(), true);
        let fb = FileCave::new(&dir).unwrap();
        fs::remove_dir(&dir).unwrap();
        // We can detect this error in case of set, due to atomic writes.
        let res = fb.set("test", &[]);
        assert_eq!(res, internal_err);
        // We can't distinguish between a missing file and a misisng directory
        // in get()/delete().
        let res = fb.get("test");
        assert_eq!(res, not_found_err);
        let res = fb.delete("test");
        assert_eq!(res, not_found_err);
    }

    #[cfg(feature = "with-rocksdb")]
    #[test]
    fn test_rocksdb_backend_simple() {
        let temp_dir = assert_fs::TempDir::new().unwrap();
        let rb = RocksDBCave::new(temp_dir.path()).unwrap();
        _test_simple(Box::new(rb));
    }

    #[cfg(feature = "with-rocksdb")]
    #[test]
    fn test_rocksdb_backend_errors() {
        let temp_dir = assert_fs::TempDir::new().unwrap();
        let internal_err = Error::Internal(anyhow!(""));

        // Test for files instead of directories.
        let empty_file = temp_dir.path().join("empty_file");
        let _ = fs::File::create(&empty_file).unwrap();
        let res = RocksDBCave::new(&empty_file);
        assert_eq!(res.is_err(), true);
        let err = res.unwrap_err();
        assert_eq!(err, internal_err);
        let msg = format!("{:?}", err);
        assert_eq!(msg.contains("Failed to create RocksDB directory"), true);

        // Test for corrupted dirs.
        let temp_dir = assert_fs::TempDir::new().unwrap();
        let corrupted_file = temp_dir.path().join("CURRENT");
        let mut file = fs::File::create(&corrupted_file).unwrap();
        file.write_all(b"corrupted").unwrap();
        let res = RocksDBCave::new(&corrupted_file);
        assert_eq!(res.is_err(), true);
        let err = res.unwrap_err();
        assert_eq!(err, internal_err);
        let msg = format!("{:?}", err);
        assert_eq!(msg.contains("Failed to create RocksDB directory"), true);

        // FIXME: Check for runtime errors.
    }
}
