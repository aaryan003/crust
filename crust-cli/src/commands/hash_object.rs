// hash-object command - compute object ID for a file or stdin

use anyhow::{anyhow, Result};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Read;
use std::path::Path;

/// Compute object ID for a file (or stdin if path is None), optionally writing to object store
pub fn cmd_hash_object(file_path: Option<&str>, write: bool, from_stdin: bool) -> Result<()> {
    let content: Vec<u8> = if from_stdin || file_path.is_none() {
        let mut buf = Vec::new();
        std::io::stdin().read_to_end(&mut buf)?;
        buf
    } else {
        let path = file_path.unwrap();
        if !Path::new(path).exists() {
            return Err(anyhow!("File not found: {}", path));
        }
        fs::read(path).map_err(|e| anyhow!("Failed to read file: {}", e))?
    };

    // Build CRUST blob object bytes
    let mut object_bytes = Vec::new();
    object_bytes.extend_from_slice(b"CRUST-OBJECT\n");
    object_bytes.extend_from_slice(b"type: blob\n");
    object_bytes.extend_from_slice(format!("size: {}\n\n", content.len()).as_bytes());
    object_bytes.extend_from_slice(&content);

    // Compute SHA256
    let mut hasher = Sha256::new();
    hasher.update(&object_bytes);
    let hash = hasher.finalize();
    let object_id = format!("{:x}", hash);

    if write {
        // Write compressed object to .crust/objects/
        if !Path::new(".crust").exists() {
            return Err(anyhow!("CLI_NO_REPOSITORY: Not in a CRUST repository"));
        }
        let dir = format!(".crust/objects/{}", &object_id[0..2]);
        fs::create_dir_all(&dir)?;
        let object_path = format!("{}/{}", dir, &object_id[2..]);
        if !Path::new(&object_path).exists() {
            let compressed = zstd::encode_all(&object_bytes[..], 3)?;
            fs::write(&object_path, compressed)?;
        }
    }

    println!("{}", object_id);
    Ok(())
}
