//! Blob (file content) implementation

use crate::ObjectId;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// A file blob in CRUST
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Blob {
    /// Raw file content
    pub content: Vec<u8>,
}

impl Blob {
    /// Create a new blob from content
    pub fn new(content: Vec<u8>) -> Self {
        Blob { content }
    }

    /// Get the content
    pub fn content(&self) -> &[u8] {
        &self.content
    }

    /// Serialize blob following CRUST object format spec
    /// Returns: header + content bytes (ready to hash)
    pub fn serialize(&self) -> Vec<u8> {
        let mut result = Vec::new();
        result.extend_from_slice(b"CRUST-OBJECT\n");
        result.extend_from_slice(b"type: blob\n");
        result.extend_from_slice(format!("size: {}\n\n", self.content.len()).as_bytes());
        result.extend_from_slice(&self.content);
        result
    }

    /// Deserialize blob from serialized bytes
    pub fn deserialize(data: &[u8]) -> crate::Result<(Self, ObjectId)> {
        // Verify header
        if !data.starts_with(b"CRUST-OBJECT\n") {
            return Err(crate::Error::InvalidObjectFormat(
                "Missing CRUST-OBJECT header".to_string(),
            ));
        }

        // Find the blank line separating header from content
        let remaining = &data[13..]; // skip "CRUST-OBJECT\n"
        let blank_line_pos = remaining
            .windows(2)
            .position(|w| w == b"\n\n")
            .ok_or_else(|| {
                crate::Error::InvalidObjectFormat("Missing blank line in header".to_string())
            })?;

        let header_part = &remaining[..blank_line_pos];
        let content_start = 13 + blank_line_pos + 2;
        let content = data[content_start..].to_vec();

        // Parse header lines
        let header_str = std::str::from_utf8(header_part)
            .map_err(|_| crate::Error::InvalidObjectFormat("Non-UTF8 header".to_string()))?;

        let mut expected_size = None;
        for line in header_str.lines() {
            if let Some(type_str) = line.strip_prefix("type: ") {
                if type_str != "blob" {
                    return Err(crate::Error::InvalidObjectFormat(format!(
                        "Expected type blob, got {}",
                        type_str
                    )));
                }
            } else if let Some(size_str) = line.strip_prefix("size: ") {
                expected_size = Some(size_str.parse::<usize>().map_err(|_| {
                    crate::Error::InvalidObjectFormat("Invalid size field".to_string())
                })?);
            }
        }

        let expected_size = expected_size
            .ok_or_else(|| crate::Error::InvalidObjectFormat("Missing size field".to_string()))?;

        if content.len() != expected_size {
            return Err(crate::Error::InvalidObjectFormat(format!(
                "Content size mismatch: expected {}, got {}",
                expected_size,
                content.len()
            )));
        }

        // Compute SHA256
        let mut hasher = Sha256::new();
        hasher.update(data);
        let hash = hasher.finalize();
        let id = ObjectId(hex::encode(hash));

        Ok((Blob { content }, id))
    }

    /// Compute object ID (SHA256) for this blob
    pub fn compute_id(&self) -> ObjectId {
        let serialized = self.serialize();
        let mut hasher = Sha256::new();
        hasher.update(&serialized);
        let hash = hasher.finalize();
        ObjectId(hex::encode(hash))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blob_creation() {
        let content = b"Hello, CRUST!".to_vec();
        let blob = Blob::new(content.clone());
        assert_eq!(blob.content(), content.as_slice());
    }

    #[test]
    fn test_blob_serialize() {
        let blob = Blob::new(b"test".to_vec());
        let serialized = blob.serialize();
        let text = String::from_utf8(serialized.clone()).unwrap();
        assert!(text.contains("CRUST-OBJECT"));
        assert!(text.contains("type: blob"));
        assert!(text.contains("size: 4"));
    }

    #[test]
    fn test_blob_round_trip() {
        let content = b"Hello, World!".to_vec();
        let blob = Blob::new(content.clone());
        let serialized = blob.serialize();

        let (deserialized, id) = Blob::deserialize(&serialized).unwrap();
        assert_eq!(deserialized.content, content);

        // ID should be deterministic
        let id2 = blob.compute_id();
        assert_eq!(id, id2);
    }

    #[test]
    fn test_empty_blob() {
        let blob = Blob::new(Vec::new());
        let serialized = blob.serialize();
        let (deserialized, _id) = Blob::deserialize(&serialized).unwrap();
        assert_eq!(deserialized.content.len(), 0);
    }
}
