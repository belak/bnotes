//! Storage abstraction for file operations
//!
//! The Storage trait provides an abstraction over filesystem operations,
//! allowing the library to be tested without touching the filesystem.
//! All paths are relative to the notes directory root.

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// Storage abstraction for file operations
///
/// All paths are relative to the notes directory. Implementations
/// handle the mapping to absolute paths or in-memory storage.
pub trait Storage {
    /// Read a file to a string
    fn read_to_string(&self, path: &Path) -> Result<String>;

    /// Write contents to a file
    fn write(&self, path: &Path, contents: &str) -> Result<()>;

    /// Check if a path exists
    fn exists(&self, path: &Path) -> bool;

    /// Check if a path is a directory
    fn is_dir(&self, path: &Path) -> bool;

    /// Read directory entries, returning relative paths
    fn read_dir(&self, path: &Path) -> Result<Vec<PathBuf>>;

    /// Create directory and all parent directories
    fn create_dir_all(&self, path: &Path) -> Result<()>;
}

/// Real filesystem storage implementation
///
/// All operations are scoped to a root directory (the notes directory).
/// Paths passed to Storage methods are interpreted relative to this root.
pub struct RealStorage {
    root: PathBuf,
}

impl RealStorage {
    /// Create a new RealStorage with the given root directory
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    /// Get the full path by joining with root
    fn full_path(&self, path: &Path) -> PathBuf {
        self.root.join(path)
    }
}

impl Storage for RealStorage {
    fn read_to_string(&self, path: &Path) -> Result<String> {
        std::fs::read_to_string(self.full_path(path))
            .with_context(|| format!("Failed to read {}", path.display()))
    }

    fn write(&self, path: &Path, contents: &str) -> Result<()> {
        std::fs::write(self.full_path(path), contents)
            .with_context(|| format!("Failed to write {}", path.display()))
    }

    fn exists(&self, path: &Path) -> bool {
        self.full_path(path).exists()
    }

    fn is_dir(&self, path: &Path) -> bool {
        self.full_path(path).is_dir()
    }

    fn read_dir(&self, path: &Path) -> Result<Vec<PathBuf>> {
        let full_path = self.full_path(path);
        let entries = std::fs::read_dir(&full_path)
            .with_context(|| format!("Failed to read directory {}", path.display()))?;

        entries
            .map(|entry| {
                entry
                    .map(|e| {
                        e.path()
                            .strip_prefix(&self.root)
                            .unwrap()
                            .to_path_buf()
                    })
                    .map_err(Into::into)
            })
            .collect()
    }

    fn create_dir_all(&self, path: &Path) -> Result<()> {
        std::fs::create_dir_all(self.full_path(path))
            .with_context(|| format!("Failed to create directory {}", path.display()))
    }
}

/// In-memory storage implementation for testing
///
/// Stores files in a HashMap, allowing tests to run without
/// touching the filesystem.
pub struct MemoryStorage {
    files: Arc<Mutex<HashMap<PathBuf, String>>>,
}

impl MemoryStorage {
    /// Create a new empty MemoryStorage
    pub fn new() -> Self {
        Self {
            files: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl Default for MemoryStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl Storage for MemoryStorage {
    fn read_to_string(&self, path: &Path) -> Result<String> {
        let files = self.files.lock().unwrap();
        files
            .get(path)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("File not found: {}", path.display()))
    }

    fn write(&self, path: &Path, contents: &str) -> Result<()> {
        let mut files = self.files.lock().unwrap();
        files.insert(path.to_path_buf(), contents.to_string());
        Ok(())
    }

    fn exists(&self, path: &Path) -> bool {
        let files = self.files.lock().unwrap();
        files.contains_key(path)
    }

    fn is_dir(&self, path: &Path) -> bool {
        let files = self.files.lock().unwrap();
        let path_str = path.to_string_lossy();
        files.keys().any(|k| {
            k.to_string_lossy().starts_with(&*path_str) && k != path
        })
    }

    fn read_dir(&self, path: &Path) -> Result<Vec<PathBuf>> {
        let files = self.files.lock().unwrap();
        let path_str = path.to_string_lossy();

        let mut entries: Vec<PathBuf> = files
            .keys()
            .filter(|k| {
                let k_str = k.to_string_lossy();
                k_str.starts_with(&*path_str) && k != &path
            })
            .cloned()
            .collect();

        entries.sort();
        Ok(entries)
    }

    fn create_dir_all(&self, _path: &Path) -> Result<()> {
        // No-op for memory storage
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_storage_write_read() {
        let storage = MemoryStorage::new();
        storage
            .write(Path::new("test.md"), "Hello, world!")
            .unwrap();

        let content = storage.read_to_string(Path::new("test.md")).unwrap();
        assert_eq!(content, "Hello, world!");
    }

    #[test]
    fn test_memory_storage_exists() {
        let storage = MemoryStorage::new();
        assert!(!storage.exists(Path::new("test.md")));

        storage
            .write(Path::new("test.md"), "content")
            .unwrap();
        assert!(storage.exists(Path::new("test.md")));
    }

    #[test]
    fn test_memory_storage_read_dir() {
        let storage = MemoryStorage::new();
        storage.write(Path::new("a.md"), "a").unwrap();
        storage.write(Path::new("b.md"), "b").unwrap();
        storage.write(Path::new("dir/c.md"), "c").unwrap();

        let entries = storage.read_dir(Path::new("")).unwrap();
        assert_eq!(entries.len(), 3);
        assert!(entries.contains(&PathBuf::from("a.md")));
        assert!(entries.contains(&PathBuf::from("b.md")));
        assert!(entries.contains(&PathBuf::from("dir/c.md")));
    }
}
