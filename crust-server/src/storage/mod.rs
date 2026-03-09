// Object storage module - disk persistence with zstd compression and CRUSTPACK format
// Implements contracts/object-format.md and contracts/crustpack-format.md

use anyhow::{anyhow, Context, Result};
use gitcore::object::{ObjectId, ObjectType};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;

/// Compression level for zstd (1-22, where 3 is standard balance)
const ZSTD_COMPRESSION_LEVEL: i32 = 3;

/// Root path for all object storage
/// Objects stored at: {base}/repos/{owner_id}/{repo_id}.crust/objects/
pub struct ObjectStore {
    base_path: PathBuf,
}

impl ObjectStore {
    /// Create a new ObjectStore with the given base path
    /// Path will be created if it doesn't exist
    pub fn new(base_path: impl AsRef<Path>) -> Result<Self> {
        let base = base_path.as_ref().to_path_buf();
        fs::create_dir_all(&base).context(format!(
            "Failed to create object store directory: {:?}",
            base
        ))?;
        Ok(ObjectStore { base_path: base })
    }

    /// Build the object directory path for a repo
    /// Format: {base}/repos/{owner_id}/{repo_id}.crust/objects/
    pub fn repo_objects_dir(&self, owner_id: &str, repo_id: &str) -> PathBuf {
        self.base_path
            .join("repos")
            .join(owner_id)
            .join(format!("{}.crust", repo_id))
            .join("objects")
    }

    /// Build the full path to an object file
    /// Format: {dir}/{id[0..2]}/{id[2..64]}
    fn object_path(&self, dir: &Path, object_id: &ObjectId) -> PathBuf {
        let id_str = object_id.as_str();
        dir.join(&id_str[0..2]).join(&id_str[2..])
    }

    /// Save a single object to disk with zstd compression
    /// Returns the ObjectId (SHA256 of the serialized object)
    pub fn save_object(&self, owner_id: &str, repo_id: &str, obj_bytes: &[u8]) -> Result<ObjectId> {
        let dir = self.repo_objects_dir(owner_id, repo_id);
        fs::create_dir_all(&dir).context("Failed to create objects directory")?;

        // Compress the object bytes
        let compressed = zstd::encode_all(obj_bytes, ZSTD_COMPRESSION_LEVEL)
            .context("Failed to compress object with zstd")?;

        // Compute SHA256 of the ORIGINAL object bytes to get the ObjectId
        let object_id = ObjectId::from_bytes(obj_bytes)
            .context("Invalid object format - failed to compute ID")?;

        // Write compressed bytes to disk
        let path = self.object_path(&dir, &object_id);
        fs::create_dir_all(path.parent().unwrap())
            .context("Failed to create object prefix directory")?;
        fs::write(&path, &compressed)
            .context(format!("Failed to write object to disk: {:?}", path))?;

        Ok(object_id)
    }

    /// Load an object from disk and decompress it
    pub fn load_object(
        &self,
        owner_id: &str,
        repo_id: &str,
        object_id: &ObjectId,
    ) -> Result<Vec<u8>> {
        let dir = self.repo_objects_dir(owner_id, repo_id);
        let path = self.object_path(&dir, object_id);

        let compressed =
            fs::read(&path).context(format!("Failed to read object from disk: {:?}", path))?;

        let decompressed =
            zstd::decode_all(&compressed[..]).context("Failed to decompress object with zstd")?;

        Ok(decompressed)
    }

    /// Check if an object exists in storage
    pub fn has_object(&self, owner_id: &str, repo_id: &str, object_id: &ObjectId) -> bool {
        let dir = self.repo_objects_dir(owner_id, repo_id);
        let path = self.object_path(&dir, object_id);
        path.exists()
    }

    /// List refs of a given type (heads or tags) for a repo.
    /// Returns a map of ref_name → commit_sha.
    pub fn list_refs(
        &self,
        owner_id: &str,
        repo_id: &str,
        ref_type: &str,
    ) -> std::collections::HashMap<String, String> {
        let refs_dir = self
            .base_path
            .join("repos")
            .join(owner_id)
            .join(format!("{}.crust", repo_id))
            .join("refs")
            .join(ref_type);

        let mut result = std::collections::HashMap::new();
        collect_refs(&refs_dir, &refs_dir, &mut result);
        result
    }

    /// Write a ref (branch or tag) for a repo.
    /// `ref_name` may be the full path like "refs/heads/main" or relative "heads/main".
    pub fn write_ref(
        &self,
        owner_id: &str,
        repo_id: &str,
        ref_name: &str,
        commit_sha: &str,
    ) -> Result<()> {
        // Normalize: strip leading "refs/" if present, so "refs/heads/main" → "heads/main"
        let normalized = ref_name
            .strip_prefix("refs/")
            .unwrap_or(ref_name);

        let ref_path = self
            .base_path
            .join("repos")
            .join(owner_id)
            .join(format!("{}.crust", repo_id))
            .join("refs")
            .join(normalized);

        if let Some(parent) = ref_path.parent() {
            fs::create_dir_all(parent).context("Failed to create ref directory")?;
        }

        fs::write(&ref_path, format!("{}\n", commit_sha))
            .context(format!("Failed to write ref: {:?}", ref_path))?;

        Ok(())
    }
}

/// Recursively collect ref files from a directory into a map.
/// Key is the path relative to `base`, value is the file content (trimmed).
fn collect_refs(
    base: &Path,
    dir: &Path,
    result: &mut std::collections::HashMap<String, String>,
) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            if let Ok(sha) = fs::read_to_string(&path) {
                let sha = sha.trim().to_string();
                // Make relative path from base
                if let Ok(relative) = path.strip_prefix(base) {
                    let name = relative.to_string_lossy().replace('\\', "/");
                    result.insert(name, sha);
                }
            }
        } else if path.is_dir() {
            collect_refs(base, &path, result);
        }
    }
}

/// CRUSTPACK format writer - serializes objects for transmission
/// Format per contracts/crustpack-format.md:
/// CRUSTPACK\n
/// version: 1\n
/// count: {count}\n
/// \n
/// [object entries]
/// {32 bytes: SHA256 of all preceding bytes}
pub struct PackWriter {
    objects: Vec<PackObject>,
}

struct PackObject {
    id: ObjectId,
    object_type: ObjectType,
    data: Vec<u8>,
}

impl PackWriter {
    pub fn new() -> Self {
        PackWriter {
            objects: Vec::new(),
        }
    }
}

impl Default for PackWriter {
    fn default() -> Self {
        Self::new()
    }
}

impl PackWriter {
    /// Add an object to the pack
    pub fn add_object(&mut self, id: ObjectId, object_type: ObjectType, data: Vec<u8>) {
        self.objects.push(PackObject {
            id,
            object_type,
            data,
        });
    }

    /// Serialize the pack to bytes including header, objects, and trailer
    pub fn serialize(&self) -> Result<Vec<u8>> {
        let mut buffer = Vec::new();

        // Write header
        buffer.extend_from_slice(b"CRUSTPACK\n");
        buffer.extend_from_slice(b"version: 1\n");
        buffer.extend_from_slice(format!("count: {}\n\n", self.objects.len()).as_bytes());

        // Write objects - note: data is raw bytes, no newline delimiter
        for obj in &self.objects {
            buffer.extend_from_slice(format!("id: {}\n", obj.id.as_str()).as_bytes());
            buffer.extend_from_slice(format!("type: {}\n", obj.object_type.as_str()).as_bytes());
            buffer.extend_from_slice(format!("size: {}\n", obj.data.len()).as_bytes());
            buffer.extend_from_slice(&obj.data);
            // NO newline after data - size field tells us exactly where it ends
        }

        // Compute SHA256 trailer of all preceding bytes
        let mut hasher = Sha256::new();
        hasher.update(&buffer);
        let hash = hasher.finalize();
        buffer.extend_from_slice(&hash);

        Ok(buffer)
    }
}

/// CRUSTPACK format reader - parses objects from transmission
pub struct PackReader;

impl PackReader {
    /// Deserialize a CRUSTPACK-formatted byte stream
    pub fn deserialize(bytes: &[u8]) -> Result<Vec<(ObjectId, ObjectType, Vec<u8>)>> {
        if bytes.len() < 32 {
            return Err(anyhow!("Pack too small - missing trailer"));
        }

        // Verify trailer (last 32 bytes)
        let (pack_bytes, trailer_bytes) = bytes.split_at(bytes.len() - 32);
        let mut hasher = Sha256::new();
        hasher.update(pack_bytes);
        let expected_hash = hasher.finalize();

        if trailer_bytes != &expected_hash[..] {
            return Err(anyhow!("Pack trailer SHA256 mismatch - corrupted data"));
        }

        // Parse header - find the blank line that separates header from objects
        let mut header_end = 0;
        for i in 0..pack_bytes.len() - 1 {
            if pack_bytes[i] == b'\n' && pack_bytes[i + 1] == b'\n' {
                header_end = i + 2;
                break;
            }
        }

        if header_end == 0 {
            return Err(anyhow!(
                "Pack header malformed - missing blank line separator"
            ));
        }

        let header_text = std::str::from_utf8(&pack_bytes[0..header_end - 2])
            .context("Pack header contains invalid UTF-8")?;

        let mut lines = header_text.lines();

        // Verify CRUSTPACK magic
        let magic = lines
            .next()
            .ok_or_else(|| anyhow!("Missing CRUSTPACK header"))?;
        if magic != "CRUSTPACK" {
            return Err(anyhow!(
                "Invalid pack header: expected 'CRUSTPACK', got '{}'",
                magic
            ));
        }

        // Verify version
        let version_line = lines
            .next()
            .ok_or_else(|| anyhow!("Missing version line"))?;
        if !version_line.starts_with("version:") {
            return Err(anyhow!("Missing or invalid version line"));
        }

        // Parse count
        let count_line = lines.next().ok_or_else(|| anyhow!("Missing count line"))?;
        let count_str = count_line
            .strip_prefix("count: ")
            .ok_or_else(|| anyhow!("Invalid count line"))?;
        let count: usize = count_str.parse().context("Count is not a valid number")?;

        // Parse objects from remaining bytes
        let mut objects = Vec::new();
        let mut pos = header_end;

        for _ in 0..count {
            // Find "id: " line
            let id_start = pos;
            let id_end = pack_bytes[pos..]
                .windows(1)
                .position(|w| w[0] == b'\n')
                .map(|i| pos + i)
                .ok_or_else(|| anyhow!("Missing newline after id line"))?;

            let id_line = std::str::from_utf8(&pack_bytes[id_start..id_end])
                .context("id line contains invalid UTF-8")?;
            let id_str = id_line
                .strip_prefix("id: ")
                .ok_or_else(|| anyhow!("Invalid id line format"))?;
            let object_id =
                ObjectId::parse(id_str).context(format!("Invalid object ID: {}", id_str))?;

            pos = id_end + 1;

            // Find "type: " line
            let type_start = pos;
            let type_end = pack_bytes[pos..]
                .windows(1)
                .position(|w| w[0] == b'\n')
                .map(|i| pos + i)
                .ok_or_else(|| anyhow!("Missing newline after type line"))?;

            let type_line = std::str::from_utf8(&pack_bytes[type_start..type_end])
                .context("type line contains invalid UTF-8")?;
            let type_str = type_line
                .strip_prefix("type: ")
                .ok_or_else(|| anyhow!("Invalid type line format"))?;
            let object_type = ObjectType::from_str(type_str)
                .context(format!("Invalid object type: {}", type_str))?;

            pos = type_end + 1;

            // Find "size: " line
            let size_start = pos;
            let size_end = pack_bytes[pos..]
                .windows(1)
                .position(|w| w[0] == b'\n')
                .map(|i| pos + i)
                .ok_or_else(|| anyhow!("Missing newline after size line"))?;

            let size_line = std::str::from_utf8(&pack_bytes[size_start..size_end])
                .context("size line contains invalid UTF-8")?;
            let size_str = size_line
                .strip_prefix("size: ")
                .ok_or_else(|| anyhow!("Invalid size line format"))?;
            let size: usize = size_str.parse().context("Size is not a valid number")?;

            pos = size_end + 1;

            // Read exactly `size` bytes of object data
            if pos + size > pack_bytes.len() {
                return Err(anyhow!("Object data extends beyond pack bounds"));
            }

            let data = pack_bytes[pos..pos + size].to_vec();
            objects.push((object_id, object_type, data));
            pos += size;
        }

        Ok(objects)
    }
}

impl ObjectStore {
    /// Collect all objects reachable from the given commit IDs via BFS graph traversal.
    ///
    /// Walks the commit graph (parents) and tree graph (subtrees + blobs) starting
    /// from each wanted commit. Skips any object already in `haves`.
    ///
    /// Returns a list of `(ObjectId, ObjectType, raw_object_bytes)` tuples ready
    /// to be added to a `PackWriter`.
    pub fn collect_reachable_objects(
        &self,
        owner_id: &str,
        repo_id: &str,
        wants: &[String],
        haves: &[String],
    ) -> Vec<(ObjectId, ObjectType, Vec<u8>)> {
        use std::collections::{HashSet, VecDeque};

        let have_set: HashSet<String> = haves.iter().cloned().collect();
        let mut visited: HashSet<String> = HashSet::new();
        let mut queue: VecDeque<(String, ObjectType)> = VecDeque::new();
        let mut result: Vec<(ObjectId, ObjectType, Vec<u8>)> = Vec::new();

        for want in wants {
            if !have_set.contains(want) && !visited.contains(want) {
                queue.push_back((want.clone(), ObjectType::Commit));
            }
        }

        while let Some((id_str, hint_type)) = queue.pop_front() {
            if visited.contains(&id_str) || have_set.contains(&id_str) {
                continue;
            }
            visited.insert(id_str.clone());

            let obj_id = match ObjectId::parse(&id_str) {
                Ok(id) => id,
                Err(_) => continue,
            };

            if !self.has_object(owner_id, repo_id, &obj_id) {
                continue;
            }

            let raw = match self.load_object(owner_id, repo_id, &obj_id) {
                Ok(data) => data,
                Err(_) => continue,
            };

            // Parse the CRUST-OBJECT header to determine actual type
            let actual_type = parse_object_type(&raw).unwrap_or(hint_type);

            // Enqueue children based on type
            match actual_type {
                ObjectType::Commit => {
                    if let Some(content) = extract_object_content_bytes(&raw) {
                        if let Ok(commit) = gitcore::Commit::deserialize(content) {
                            // Enqueue tree
                            let tree_id = commit.tree.as_hex().to_string();
                            if !visited.contains(&tree_id) && !have_set.contains(&tree_id) {
                                queue.push_back((tree_id, ObjectType::Tree));
                            }
                            // Enqueue parents
                            for parent in &commit.parents {
                                let parent_id = parent.as_hex().to_string();
                                if !visited.contains(&parent_id)
                                    && !have_set.contains(&parent_id)
                                {
                                    queue.push_back((parent_id, ObjectType::Commit));
                                }
                            }
                        }
                    }
                }
                ObjectType::Tree => {
                    if let Some(content) = extract_object_content_bytes(&raw) {
                        if let Ok(tree) = gitcore::Tree::deserialize(content) {
                            for entry in tree.entries() {
                                let entry_id = entry.id.as_hex().to_string();
                                if !visited.contains(&entry_id) && !have_set.contains(&entry_id) {
                                    let child_type = if entry.mode == "40000" {
                                        ObjectType::Tree
                                    } else {
                                        ObjectType::Blob
                                    };
                                    queue.push_back((entry_id, child_type));
                                }
                            }
                        }
                    }
                }
                ObjectType::Blob | ObjectType::Tag => {
                    // No children to traverse
                }
            }

            result.push((obj_id, actual_type, raw));
        }

        result
    }
}

/// Parse the `type:` header line from a raw CRUST-OBJECT byte slice.
fn parse_object_type(raw: &[u8]) -> Option<ObjectType> {
    // Format: "CRUST-OBJECT\ntype: {type}\n..."
    let text = std::str::from_utf8(raw).ok()?;
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("type: ") {
            return match rest.trim() {
                "blob" => Some(ObjectType::Blob),
                "tree" => Some(ObjectType::Tree),
                "commit" => Some(ObjectType::Commit),
                "tag" => Some(ObjectType::Tag),
                _ => None,
            };
        }
        // Stop searching after the blank line that ends headers
        if line.is_empty() {
            break;
        }
    }
    None
}

/// Extract the content bytes after the CRUST-OBJECT header separator (`\n\n`).
fn extract_object_content_bytes(raw: &[u8]) -> Option<&[u8]> {
    // Find "\n\n" which separates header from content
    raw.windows(2)
        .position(|w| w == b"\n\n")
        .map(|pos| &raw[pos + 2..])
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_object_store_roundtrip() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let store = ObjectStore::new(temp_dir.path())?;

        // Create test object data
        let test_data = b"This is test object content";

        // Save object
        let obj_id = store.save_object("test_owner", "test_repo", test_data)?;

        // Verify it was saved
        assert!(store.has_object("test_owner", "test_repo", &obj_id));

        // Load and verify
        let loaded = store.load_object("test_owner", "test_repo", &obj_id)?;
        assert_eq!(loaded, test_data);

        Ok(())
    }

    #[test]
    fn test_object_store_compression() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let store = ObjectStore::new(temp_dir.path())?;

        // Create larger test data that will compress well
        let test_data = b"Lorem ipsum dolor sit amet, consectetur adipiscing elit. ".repeat(100);

        let obj_id = store.save_object("test_owner", "test_repo", &test_data)?;
        let loaded = store.load_object("test_owner", "test_repo", &obj_id)?;
        assert_eq!(loaded, test_data);

        // Verify that compressed file is smaller than original
        let dir = store.repo_objects_dir("test_owner", "test_repo");
        let path = store.object_path(&dir, &obj_id);
        let compressed_size = fs::metadata(&path)?.len() as usize;
        assert!(compressed_size < test_data.len());

        Ok(())
    }

    #[test]
    fn test_pack_writer_basic() -> Result<()> {
        let mut writer = PackWriter::new();

        // Add test objects
        let id1_str = "a".repeat(64);
        let id2_str = "b".repeat(64);
        let id1 = ObjectId::parse(&id1_str)?;
        let id2 = ObjectId::parse(&id2_str)?;

        writer.add_object(id1, ObjectType::Blob, b"blob data".to_vec());
        writer.add_object(id2, ObjectType::Tree, b"tree data".to_vec());

        let packed = writer.serialize()?;

        // Verify header is present (header should be valid UTF-8)
        let header_part = std::str::from_utf8(&packed[0..50])?;
        assert!(header_part.contains("CRUSTPACK"));
        assert!(header_part.contains("count: 2"));

        // Verify trailer exists (last 32 bytes)
        assert!(packed.len() > 32);

        // Verify structure: should have header + object entries + 32-byte trailer
        assert!(packed.len() > 100); // At least header (26 bytes) + entries + trailer

        Ok(())
    }

    #[test]
    fn test_pack_reader_roundtrip() -> Result<()> {
        let mut writer = PackWriter::new();

        let id1_str = "c".repeat(64);
        let id2_str = "d".repeat(64);
        let id1 = ObjectId::parse(&id1_str)?;
        let id2 = ObjectId::parse(&id2_str)?;
        let data1 = b"first object".to_vec();
        let data2 = b"second object".to_vec();

        writer.add_object(id1.clone(), ObjectType::Commit, data1.clone());
        writer.add_object(id2.clone(), ObjectType::Tag, data2.clone());

        let packed = writer.serialize()?;
        let unpacked = PackReader::deserialize(&packed)?;

        assert_eq!(unpacked.len(), 2);
        assert_eq!(unpacked[0].0, id1);
        assert_eq!(unpacked[0].1, ObjectType::Commit);
        assert_eq!(unpacked[0].2, data1);
        assert_eq!(unpacked[1].0, id2);
        assert_eq!(unpacked[1].1, ObjectType::Tag);
        assert_eq!(unpacked[1].2, data2);

        Ok(())
    }

    #[test]
    fn test_pack_corruption_detection() -> Result<()> {
        let mut writer = PackWriter::new();
        let id_str = "e".repeat(64);
        let id = ObjectId::parse(&id_str)?;
        writer.add_object(id, ObjectType::Blob, b"data".to_vec());

        let mut packed = writer.serialize()?;

        // Corrupt the trailer (flip a bit)
        let last_byte = packed.len() - 1;
        packed[last_byte] ^= 0x01;

        // Should fail trailer verification
        let result = PackReader::deserialize(&packed);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("trailer"));

        Ok(())
    }
}
