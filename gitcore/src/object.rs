//! Core object types for CRUST

use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// Unique identifier for a CRUST object (SHA256 hash)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ObjectId(pub String); // 64 lowercase hex chars

/// Type of a CRUST object
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ObjectType {
    /// File content
    #[serde(rename = "blob")]
    Blob,
    /// Directory listing
    #[serde(rename = "tree")]
    Tree,
    /// Commit with parent references
    #[serde(rename = "commit")]
    Commit,
    /// Annotated tag
    #[serde(rename = "tag")]
    Tag,
}

/// Generic CRUST object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Object {
    /// Object type
    pub object_type: ObjectType,
    /// Object ID (SHA256)
    pub id: ObjectId,
    /// Raw content
    pub content: Vec<u8>,
}

impl ObjectId {
    /// Create a new ObjectId from a hex string
    pub fn from_hex(hex: &str) -> crate::Result<Self> {
        if hex.len() != 64 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(crate::Error::InvalidHash(hex.to_string()));
        }
        Ok(ObjectId(hex.to_lowercase()))
    }

    /// Parse ObjectId from a hex string (alias for from_hex for compatibility)
    pub fn parse(hex: &str) -> crate::Result<Self> {
        Self::from_hex(hex)
    }

    /// Create an ObjectId by computing SHA256 of raw bytes
    pub fn from_bytes(data: &[u8]) -> crate::Result<Self> {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(data);
        let hash = hasher.finalize();
        let hex_str = format!("{:x}", hash);
        ObjectId::from_hex(&hex_str)
    }

    /// Get the hex string representation
    pub fn as_hex(&self) -> &str {
        &self.0
    }

    /// Get the hex string representation (alias)
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl ObjectType {
    /// Get string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            ObjectType::Blob => "blob",
            ObjectType::Tree => "tree",
            ObjectType::Commit => "commit",
            ObjectType::Tag => "tag",
        }
    }
}

impl FromStr for ObjectType {
    type Err = crate::Error;

    /// Parse from string
    fn from_str(s: &str) -> crate::Result<Self> {
        match s {
            "blob" => Ok(ObjectType::Blob),
            "tree" => Ok(ObjectType::Tree),
            "commit" => Ok(ObjectType::Commit),
            "tag" => Ok(ObjectType::Tag),
            _ => Err(crate::Error::InvalidObjectFormat(format!(
                "Unknown object type: {}",
                s
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_object_id_from_hex() {
        let id = ObjectId::from_hex("356a192b7913b04c54574d18c28d46e6395428ab");
        assert!(id.is_err()); // too short

        let id =
            ObjectId::from_hex("356a192b7913b04c54574d18c28d46e6395428ab356a192b7913b04c54574d18");
        assert!(id.is_ok());
    }

    #[test]
    fn test_object_type_str() {
        assert_eq!(ObjectType::Blob.as_str(), "blob");
        assert_eq!(ObjectType::Tree.as_str(), "tree");
        assert_eq!(ObjectType::Commit.as_str(), "commit");
        assert_eq!(ObjectType::Tag.as_str(), "tag");
    }
}
