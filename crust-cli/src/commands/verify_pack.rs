// verify-pack command - validate object storage integrity

use anyhow::{anyhow, Result};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

/// Check integrity of all objects in .crust/objects/ (or a specific path for compatibility)
pub fn cmd_verify_pack(verbose: bool, path: Option<&str>) -> Result<()> {
    // If a specific path was given, validate it exists
    if let Some(p) = path {
        if !Path::new(p).exists() {
            return Err(anyhow!("OBJECT_NOT_FOUND: Path not found: {}", p));
        }
    }

    // Check if we're in a repo
    if !Path::new(".crust").exists() {
        return Err(anyhow!("CLI_NO_REPOSITORY: Not in a CRUST repository"));
    }

    let objects_dir = if let Some(p) = path {
        p.to_string()
    } else {
        ".crust/objects".to_string()
    };

    if !Path::new(&objects_dir).exists() {
        println!("No objects directory found");
        return Ok(());
    }

    let mut object_count = 0;
    let mut corrupted = Vec::new();

    // Walk through all objects: .crust/objects/{2char}/{remaining}
    for entry in
        fs::read_dir(objects_dir).map_err(|e| anyhow!("Failed to read objects directory: {}", e))?
    {
        let entry = entry.map_err(|e| anyhow!("Failed to read entry: {}", e))?;
        let path = entry.path();

        // Skip if not a directory (the {2char} directories)
        if !path.is_dir() {
            continue;
        }

        // Get the 2-char prefix
        let prefix = path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| anyhow!("Invalid directory name"))?;

        // Walk the subdirectory for individual objects
        for sub_entry in
            fs::read_dir(&path).map_err(|e| anyhow!("Failed to read subdirectory: {}", e))?
        {
            let sub_entry = sub_entry.map_err(|e| anyhow!("Failed to read entry: {}", e))?;
            let obj_path = sub_entry.path();

            // Skip if it's a directory
            if obj_path.is_dir() {
                continue;
            }

            let obj_name = obj_path
                .file_name()
                .and_then(|n| n.to_str())
                .ok_or_else(|| anyhow!("Invalid object name"))?;

            object_count += 1;
            let full_id = format!("{}{}", prefix, obj_name);

            // Verify the object
            match verify_object(&obj_path, &full_id) {
                Ok(info) => {
                    if verbose {
                        println!("{} {} {}", full_id, info.obj_type, info.size);
                    }
                }
                Err(e) => {
                    corrupted.push((full_id, e.to_string()));
                }
            }
        }
    }

    // Print results
    if object_count == 0 {
        println!("No objects found");
        return Ok(());
    }

    println!("Verifying {} objects...", object_count);

    if corrupted.is_empty() {
        println!("All objects OK");
        Ok(())
    } else {
        let count = corrupted.len();
        println!("Found {} corrupted objects:", count);
        for (id, reason) in corrupted {
            println!("  {} — {}", id, reason);
        }
        Err(anyhow!(
            "OBJECT_CORRUPT: {} objects failed verification",
            count
        ))
    }
}

struct ObjectInfo {
    obj_type: String,
    size: usize,
}

/// Verify a single object by decompressing and validating header
fn verify_object(path: &Path, expected_id: &str) -> Result<ObjectInfo> {
    // Read compressed object
    let compressed = fs::read(path).map_err(|e| anyhow!("Failed to read object: {}", e))?;

    // Decompress
    let decompressed =
        zstd::decode_all(&compressed[..]).map_err(|_| anyhow!("Decompression failed"))?;

    // Verify header format
    if !decompressed.starts_with(b"CRUST-OBJECT\n") {
        return Err(anyhow!("Invalid CRUST-OBJECT header"));
    }

    // Parse header to extract type and size
    let text = String::from_utf8_lossy(&decompressed);
    let mut lines = text.lines();

    // Skip "CRUST-OBJECT"
    let _ = lines.next();

    let mut object_type = "";
    let mut declared_size = None;

    for line in &mut lines {
        if line.is_empty() {
            break; // End of header
        }

        if let Some(type_str) = line.strip_prefix("type: ") {
            object_type = type_str;
        } else if let Some(size_str) = line.strip_prefix("size: ") {
            declared_size = Some(size_str.parse::<usize>().unwrap_or(0));
        }
    }

    // Verify we have both type and size
    if object_type.is_empty() {
        return Err(anyhow!("Missing object type"));
    }

    if declared_size.is_none() {
        return Err(anyhow!("Missing size field"));
    }

    // Compute SHA256 to verify object ID matches
    let mut hasher = Sha256::new();
    hasher.update(&decompressed);
    let hash = hasher.finalize();
    let computed_id = format!("{:x}", hash);

    if computed_id != expected_id {
        return Err(anyhow!(
            "SHA256 mismatch: computed {}, expected {}",
            computed_id,
            expected_id
        ));
    }

    Ok(ObjectInfo {
        obj_type: object_type.to_string(),
        size: declared_size.unwrap_or(0),
    })
}
