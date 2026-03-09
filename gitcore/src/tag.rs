//! Tag implementation

use crate::{ObjectId, ObjectType};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// An annotated tag in CRUST
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    /// Object being tagged
    pub object: ObjectId,
    /// Object type
    pub object_type: ObjectType,
    /// Tag name
    pub name: String,
    /// Tagger name and email
    pub tagger: String,
    /// Tag timestamp (Unix seconds)
    pub timestamp: i64,
    /// Timezone offset (e.g., "+0000")
    pub tz_offset: String,
    /// Tag message
    pub message: String,
}

impl Tag {
    /// Create a new tag
    pub fn new(
        object: ObjectId,
        object_type: ObjectType,
        name: String,
        tagger: String,
        timestamp: i64,
        tz_offset: String,
        message: String,
    ) -> Self {
        Tag {
            object,
            object_type,
            name,
            tagger,
            timestamp,
            tz_offset,
            message,
        }
    }

    /// Serialize tag in text format per spec
    /// Format:
    /// object {sha256_hex}\n
    /// type {blob|tree|commit|tag}\n
    /// tag {tag_name}\n
    /// tagger {name} <{email}> {unix_timestamp} {tz_offset}\n
    /// \n
    /// {message}
    pub fn serialize(&self) -> Vec<u8> {
        let mut result = Vec::new();

        result.extend_from_slice(format!("object {}\n", self.object.as_hex()).as_bytes());
        result.extend_from_slice(format!("type {}\n", self.object_type.as_str()).as_bytes());
        result.extend_from_slice(format!("tag {}\n", self.name).as_bytes());

        result.extend_from_slice(format!("tagger {} {}\n", self.tagger, self.timestamp).as_bytes());
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
        result.extend_from_slice(b"type: tag\n");
        result.extend_from_slice(format!("size: {}\n\n", content.len()).as_bytes());
        result.extend_from_slice(&content);
        result
    }

    /// Deserialize tag from text content (without header)
    pub fn deserialize(data: &[u8]) -> crate::Result<Self> {
        let text = std::str::from_utf8(data).map_err(|_| {
            crate::Error::InvalidObjectFormat("Tag content is not valid UTF-8".to_string())
        })?;

        let lines: Vec<&str> = text.lines().collect();
        let mut object = None;
        let mut object_type = None;
        let mut name = None;
        let mut tagger = None;
        let mut timestamp = 0i64;
        let mut tz_offset = String::new();
        let mut message_start = 0;

        for (idx, line) in lines.iter().enumerate() {
            if line.is_empty() {
                message_start = idx + 1;
                break;
            }

            if let Some(rest) = line.strip_prefix("object ") {
                object = Some(ObjectId::from_hex(rest)?);
            } else if let Some(rest) = line.strip_prefix("type ") {
                object_type = Some(rest.parse()?);
            } else if let Some(rest) = line.strip_prefix("tag ") {
                name = Some(rest.to_string());
            } else if let Some(rest) = line.strip_prefix("tagger ") {
                if let Some(last_space) = rest.rfind(' ') {
                    if let Some(second_last_space) = rest[..last_space].rfind(' ') {
                        let name_email = &rest[..second_last_space];
                        let ts_str = &rest[second_last_space + 1..last_space];
                        timestamp = ts_str.parse().map_err(|_| {
                            crate::Error::InvalidObjectFormat("Invalid timestamp".to_string())
                        })?;
                        tz_offset = rest[last_space + 1..].to_string();
                        tagger = Some(name_email.to_string());
                    }
                }
            }
        }

        let message = if message_start < lines.len() {
            lines[message_start..].join("\n")
        } else {
            String::new()
        };

        let object = object.ok_or_else(|| {
            crate::Error::InvalidObjectFormat("Missing object in tag".to_string())
        })?;

        let object_type = object_type
            .ok_or_else(|| crate::Error::InvalidObjectFormat("Missing type in tag".to_string()))?;

        let name =
            name.ok_or_else(|| crate::Error::InvalidObjectFormat("Missing tag name".to_string()))?;

        let tagger = tagger.ok_or_else(|| {
            crate::Error::InvalidObjectFormat("Missing tagger in tag".to_string())
        })?;

        Ok(Tag {
            object,
            object_type,
            name,
            tagger,
            timestamp,
            tz_offset,
            message,
        })
    }

    /// Compute object ID for this tag
    pub fn compute_id(&self) -> ObjectId {
        let serialized = self.serialize_object();
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
    fn test_tag_creation() {
        let tag = Tag::new(
            ObjectId::from_hex("356a192b7913b04c54574d18c28d46e6395428ab356a192b7913b04c54574d18")
                .unwrap(),
            ObjectType::Commit,
            "v0.1.0".to_string(),
            "Alice <alice@example.com>".to_string(),
            1704067200,
            "+0000".to_string(),
            "Release v0.1.0".to_string(),
        );

        assert_eq!(tag.name, "v0.1.0");
        assert_eq!(tag.message, "Release v0.1.0");
    }

    #[test]
    fn test_tag_serialize() {
        let tag = Tag::new(
            ObjectId::from_hex("356a192b7913b04c54574d18c28d46e6395428ab356a192b7913b04c54574d18")
                .unwrap(),
            ObjectType::Commit,
            "v0.2.0".to_string(),
            "Bob <bob@example.com>".to_string(),
            1704067300,
            "+0000".to_string(),
            "Version 0.2.0".to_string(),
        );

        let serialized = tag.serialize();
        let text = std::str::from_utf8(&serialized).unwrap();

        assert!(text.contains("object 356a192b"));
        assert!(text.contains("type commit"));
        assert!(text.contains("tag v0.2.0"));
        assert!(text.contains("Version 0.2.0"));
    }
}
