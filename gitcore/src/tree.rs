//! Tree (directory) implementation

use crate::ObjectId;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// A directory tree in CRUST
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tree {
    /// Tree entries sorted by name
    pub entries: Vec<TreeEntry>,
}

/// An entry in a tree
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeEntry {
    /// Entry mode (file/directory) as decimal string
    pub mode: String,
    /// Entry name
    pub name: String,
    /// Object ID (SHA256)
    pub id: ObjectId,
}

impl Tree {
    /// Create a new tree
    pub fn new(mut entries: Vec<TreeEntry>) -> crate::Result<Self> {
        // Sort entries by name
        entries.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(Tree { entries })
    }

    /// Get entries
    pub fn entries(&self) -> &[TreeEntry] {
        &self.entries
    }

    /// Serialize tree in binary format following CRUST spec
    /// Format: {mode_ascii} {name_utf8}\0{32_raw_sha256_bytes}
    pub fn serialize(&self) -> crate::Result<Vec<u8>> {
        let mut result = Vec::new();

        for entry in &self.entries {
            // Mode as ASCII decimal (e.g., "100644")
            result.extend_from_slice(entry.mode.as_bytes());
            result.push(b' ');

            // Name as UTF-8
            result.extend_from_slice(entry.name.as_bytes());
            result.push(0); // null byte

            // SHA256 as raw 32 bytes
            let sha_hex = entry.id.as_hex();
            let sha_bytes = hex::decode(sha_hex).map_err(|_| {
                crate::Error::InvalidObjectFormat("Invalid SHA256 in tree entry".to_string())
            })?;

            if sha_bytes.len() != 32 {
                return Err(crate::Error::InvalidObjectFormat(
                    "SHA256 must be 32 bytes".to_string(),
                ));
            }
            result.extend_from_slice(&sha_bytes);
        }

        Ok(result)
    }

    /// Full serialization including CRUST object header
    pub fn serialize_object(&self) -> crate::Result<Vec<u8>> {
        let content = self.serialize()?;
        let mut result = Vec::new();
        result.extend_from_slice(b"CRUST-OBJECT\n");
        result.extend_from_slice(b"type: tree\n");
        result.extend_from_slice(format!("size: {}\n\n", content.len()).as_bytes());
        result.extend_from_slice(&content);
        Ok(result)
    }

    /// Deserialize tree from binary content (without header)
    pub fn deserialize(data: &[u8]) -> crate::Result<Self> {
        let mut entries = Vec::new();
        let mut pos = 0;

        while pos < data.len() {
            // Parse mode (ASCII until space)
            let space_pos = data[pos..].iter().position(|&b| b == b' ').ok_or_else(|| {
                crate::Error::InvalidObjectFormat("Missing space in tree entry".to_string())
            })?;

            let mode = std::str::from_utf8(&data[pos..pos + space_pos])
                .map_err(|_| {
                    crate::Error::InvalidObjectFormat("Non-UTF8 mode in tree entry".to_string())
                })?
                .to_string();

            pos += space_pos + 1; // skip mode and space

            // Parse name (UTF-8 until null byte)
            let null_pos = data[pos..].iter().position(|&b| b == 0).ok_or_else(|| {
                crate::Error::InvalidObjectFormat("Missing null byte in tree entry".to_string())
            })?;

            let name = std::str::from_utf8(&data[pos..pos + null_pos])
                .map_err(|_| {
                    crate::Error::InvalidObjectFormat("Non-UTF8 name in tree entry".to_string())
                })?
                .to_string();

            pos += null_pos + 1; // skip name and null byte

            // Parse SHA256 (32 raw bytes)
            if pos + 32 > data.len() {
                return Err(crate::Error::InvalidObjectFormat(
                    "Truncated SHA256 in tree entry".to_string(),
                ));
            }

            let sha_bytes = &data[pos..pos + 32];
            let id = ObjectId(hex::encode(sha_bytes));
            pos += 32;

            entries.push(TreeEntry { mode, name, id });
        }

        Tree::new(entries)
    }

    /// Compute object ID for this tree
    pub fn compute_id(&self) -> crate::Result<ObjectId> {
        let serialized = self.serialize_object()?;
        let mut hasher = Sha256::new();
        hasher.update(&serialized);
        let hash = hasher.finalize();
        Ok(ObjectId(hex::encode(hash)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tree_sorting() {
        let entries = vec![
            TreeEntry {
                mode: "100644".to_string(),
                name: "z_file.txt".to_string(),
                id: ObjectId::from_hex(
                    "356a192b7913b04c54574d18c28d46e6395428ab356a192b7913b04c54574d18",
                )
                .unwrap(),
            },
            TreeEntry {
                mode: "100644".to_string(),
                name: "a_file.txt".to_string(),
                id: ObjectId::from_hex(
                    "356a192b7913b04c54574d18c28d46e6395428ab356a192b7913b04c54574d18",
                )
                .unwrap(),
            },
        ];

        let tree = Tree::new(entries).unwrap();
        assert_eq!(tree.entries[0].name, "a_file.txt");
        assert_eq!(tree.entries[1].name, "z_file.txt");
    }

    #[test]
    fn test_tree_serialize_deserialize() {
        let entries = vec![TreeEntry {
            mode: "100644".to_string(),
            name: "README.md".to_string(),
            id: ObjectId::from_hex(
                "356a192b7913b04c54574d18c28d46e6395428ab356a192b7913b04c54574d18",
            )
            .unwrap(),
        }];

        let tree = Tree::new(entries).unwrap();
        let serialized = tree.serialize().unwrap();
        let deserialized = Tree::deserialize(&serialized).unwrap();

        assert_eq!(deserialized.entries.len(), 1);
        assert_eq!(deserialized.entries[0].name, "README.md");
        assert_eq!(deserialized.entries[0].mode, "100644");
    }

    #[test]
    fn test_tree_binary_format() {
        let entries = vec![TreeEntry {
            mode: "100644".to_string(),
            name: "test".to_string(),
            id: ObjectId::from_hex(
                "0000000000000000000000000000000000000000000000000000000000000000",
            )
            .unwrap(),
        }];

        let tree = Tree::new(entries).unwrap();
        let serialized = tree.serialize().unwrap();

        // Should contain mode as ASCII, space, name, null byte, then 32 raw bytes
        assert!(serialized.starts_with(b"100644 test\0"));
        assert_eq!(serialized.len(), 6 + 1 + 4 + 1 + 32); // mode + space + name + null + sha
    }
}
