// Index file handling - manages .crust/index staging area

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// An entry in the index (staging area)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexEntry {
    /// File path relative to repo root
    pub path: String,
    /// SHA256 blob ID
    pub blob_id: String,
    /// File size in bytes
    pub size: u64,
    /// Last modified time (unix timestamp)
    pub mtime: u64,
}

/// The index file structure (stores staged changes)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Index {
    /// Entries staged for commit
    pub entries: Vec<IndexEntry>,
}

impl Index {
    /// Create an empty index
    pub fn new() -> Self {
        Index {
            entries: Vec::new(),
        }
    }

    /// Load index from .crust/index file
    pub fn load(repo_root: &str) -> Result<Self> {
        let index_path = format!("{}/.crust/index", repo_root);
        if !Path::new(&index_path).exists() {
            return Ok(Index::new());
        }

        let contents = fs::read_to_string(&index_path)?;
        if contents.trim().is_empty() {
            return Ok(Index::new());
        }

        let index: Index = serde_json::from_str(&contents)?;
        Ok(index)
    }

    /// Save index to .crust/index file
    pub fn save(&self, repo_root: &str) -> Result<()> {
        let index_path = format!("{}/.crust/index", repo_root);
        let json = serde_json::to_string_pretty(self)?;
        fs::write(&index_path, json)?;
        Ok(())
    }

    /// Add or update an entry in the index
    pub fn add_entry(&mut self, entry: IndexEntry) {
        // Remove existing entry with same path
        self.entries.retain(|e| e.path != entry.path);
        // Add new entry
        self.entries.push(entry);
        // Sort by path for consistency
        self.entries.sort_by(|a, b| a.path.cmp(&b.path));
    }

    /// Remove an entry from the index
    pub fn remove_entry(&mut self, path: &str) {
        self.entries.retain(|e| e.path != path);
    }

    /// Get an entry by path
    pub fn get_entry(&self, path: &str) -> Option<&IndexEntry> {
        self.entries.iter().find(|e| e.path == path)
    }

    /// Get all entries
    pub fn entries(&self) -> &[IndexEntry] {
        &self.entries
    }

    /// Check if index has any entries
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl Default for Index {
    fn default() -> Self {
        Self::new()
    }
}
