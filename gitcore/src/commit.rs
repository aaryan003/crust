//! Commit implementation

use crate::ObjectId;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// A commit in CRUST
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Commit {
    /// Tree object ID
    pub tree: ObjectId,
    /// Parent commit IDs (can be multiple for merge commits)
    pub parents: Vec<ObjectId>,
    /// Author name and email
    pub author: String,
    /// Committer name and email
    pub committer: String,
    /// Commit timestamp (Unix seconds)
    pub timestamp: i64,
    /// Timezone offset (e.g., "+0000", "-0500")
    pub tz_offset: String,
    /// Commit message
    pub message: String,
}

impl Commit {
    /// Create a new commit
    pub fn new(
        tree: ObjectId,
        parents: Vec<ObjectId>,
        author: String,
        committer: String,
        timestamp: i64,
        tz_offset: String,
        message: String,
    ) -> Self {
        Commit {
            tree,
            parents,
            author,
            committer,
            timestamp,
            tz_offset,
            message,
        }
    }

    /// Serialize commit in text format per spec
    /// Format:
    /// tree {sha256_hex}\n
    /// parent {sha256_hex}\n  [one per parent, none for root]
    /// author {name} <{email}> {unix_timestamp} {tz_offset}\n
    /// committer {name} <{email}> {unix_timestamp} {tz_offset}\n
    /// \n
    /// {message}
    pub fn serialize(&self) -> Vec<u8> {
        let mut result = Vec::new();

        result.extend_from_slice(format!("tree {}\n", self.tree.as_hex()).as_bytes());

        for parent in &self.parents {
            result.extend_from_slice(format!("parent {}\n", parent.as_hex()).as_bytes());
        }

        result.extend_from_slice(format!("author {} {}\n", self.author, self.timestamp).as_bytes());
        result.extend_from_slice(self.tz_offset.as_bytes());
        result.push(b'\n');

        result.extend_from_slice(
            format!("committer {} {}\n", self.committer, self.timestamp).as_bytes(),
        );
        result.extend_from_slice(self.tz_offset.as_bytes());
        result.push(b'\n');

        result.extend_from_slice(b"\n");
        result.extend_from_slice(self.message.as_bytes());

        result
    }

    /// Full serialization including CRUST object header
    pub fn serialize_object(&self) -> Vec<u8> {
        let content = self.serialize();
        let mut result = Vec::new();
        result.extend_from_slice(b"CRUST-OBJECT\n");
        result.extend_from_slice(b"type: commit\n");
        result.extend_from_slice(format!("size: {}\n\n", content.len()).as_bytes());
        result.extend_from_slice(&content);
        result
    }

    /// Deserialize commit from text content (without header)
    pub fn deserialize(data: &[u8]) -> crate::Result<Self> {
        let text = std::str::from_utf8(data).map_err(|_| {
            crate::Error::InvalidObjectFormat("Commit content is not valid UTF-8".to_string())
        })?;

        let lines: Vec<&str> = text.lines().collect();
        let mut tree = None;
        let mut parents = Vec::new();
        let mut author = None;
        let mut committer = None;
        let mut timestamp = 0i64;
        let mut tz_offset = String::new();
        let mut message_start = 0;

        for (idx, line) in lines.iter().enumerate() {
            if line.is_empty() {
                // Blank line marks start of message
                message_start = idx + 1;
                break;
            }

            if let Some(rest) = line.strip_prefix("tree ") {
                tree = Some(ObjectId::from_hex(rest)?);
            } else if let Some(rest) = line.strip_prefix("parent ") {
                parents.push(ObjectId::from_hex(rest)?);
            } else if let Some(rest) = line.strip_prefix("author ") {
                // Parse: name <email> timestamp tzoffset
                if let Some(last_space) = rest.rfind(' ') {
                    if let Some(second_last_space) = rest[..last_space].rfind(' ') {
                        let name_email = &rest[..second_last_space];
                        let ts_str = &rest[second_last_space + 1..last_space];
                        timestamp = ts_str.parse().map_err(|_| {
                            crate::Error::InvalidObjectFormat("Invalid timestamp".to_string())
                        })?;
                        tz_offset = rest[last_space + 1..].to_string();
                        author = Some(name_email.to_string());
                    }
                }
            } else if let Some(rest) = line.strip_prefix("committer ") {
                // Similar parsing
                if let Some(last_space) = rest.rfind(' ') {
                    if let Some(second_last_space) = rest[..last_space].rfind(' ') {
                        let name_email = &rest[..second_last_space];
                        committer = Some(name_email.to_string());
                    }
                }
            }
        }

        // Reconstruct message from remaining lines
        let message = if message_start < lines.len() {
            lines[message_start..].join("\n")
        } else {
            String::new()
        };

        let tree = tree.ok_or_else(|| {
            crate::Error::InvalidObjectFormat("Missing tree in commit".to_string())
        })?;

        let author = author.ok_or_else(|| {
            crate::Error::InvalidObjectFormat("Missing author in commit".to_string())
        })?;

        let committer = committer.ok_or_else(|| {
            crate::Error::InvalidObjectFormat("Missing committer in commit".to_string())
        })?;

        Ok(Commit {
            tree,
            parents,
            author,
            committer,
            timestamp,
            tz_offset,
            message,
        })
    }

    /// Compute object ID for this commit
    pub fn compute_id(&self) -> ObjectId {
        let serialized = self.serialize_object();
        let mut hasher = Sha256::new();
        hasher.update(&serialized);
        let hash = hasher.finalize();
        ObjectId(hex::encode(hash))
    }

    /// Check if this is a root commit (no parents)
    pub fn is_root(&self) -> bool {
        self.parents.is_empty()
    }

    /// Check if this is a merge commit (2+ parents)
    pub fn is_merge(&self) -> bool {
        self.parents.len() >= 2
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_commit_creation() {
        let commit = Commit::new(
            ObjectId::from_hex("356a192b7913b04c54574d18c28d46e6395428ab356a192b7913b04c54574d18")
                .unwrap(),
            vec![],
            "Alice <alice@example.com>".to_string(),
            "Alice <alice@example.com>".to_string(),
            1704067200,
            "+0000".to_string(),
            "Initial commit".to_string(),
        );

        assert_eq!(commit.message, "Initial commit");
        assert_eq!(commit.parents.len(), 0);
        assert!(commit.is_root());
    }

    #[test]
    fn test_commit_serialize() {
        let commit = Commit::new(
            ObjectId::from_hex("356a192b7913b04c54574d18c28d46e6395428ab356a192b7913b04c54574d18")
                .unwrap(),
            vec![],
            "Alice <alice@example.com>".to_string(),
            "Alice <alice@example.com>".to_string(),
            1704067200,
            "+0000".to_string(),
            "Test commit".to_string(),
        );

        let serialized = commit.serialize();
        let text = std::str::from_utf8(&serialized).unwrap();

        assert!(text.contains("tree 356a192b"));
        assert!(text.contains("author Alice <alice@example.com>"));
        assert!(text.contains("Test commit"));
    }

    #[test]
    fn test_merge_commit() {
        let parent1 =
            ObjectId::from_hex("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa00")
                .unwrap();
        let parent2 =
            ObjectId::from_hex("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb00")
                .unwrap();

        let commit = Commit::new(
            ObjectId::from_hex("356a192b7913b04c54574d18c28d46e6395428ab356a192b7913b04c54574d18")
                .unwrap(),
            vec![parent1.clone(), parent2.clone()],
            "Bob <bob@example.com>".to_string(),
            "Bob <bob@example.com>".to_string(),
            1704067300,
            "+0000".to_string(),
            "Merge feature".to_string(),
        );

        assert!(commit.is_merge());
        assert_eq!(commit.parents.len(), 2);
    }
}
