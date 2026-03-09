// cat-object command - decompress and print object content

use anyhow::{anyhow, Result};
use std::fs;
use std::path::Path;

/// Decompress and print object content to stdout
/// show_type=true  → print only the type (e.g. "blob")
/// show_size=true  → print only the size in bytes
/// both false      → print the full raw content after the header
pub fn cmd_cat_object(id: &str, show_type: bool, show_size: bool) -> Result<()> {
    // Validate object ID format (64 hex chars)
    if id.len() != 64 || !id.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(anyhow!(
            "VALIDATE_INVALID_FORMAT: Object ID must be 64 hex characters"
        ));
    }

    // Check if we're in a repo
    if !Path::new(".crust").exists() {
        return Err(anyhow!("CLI_NO_REPOSITORY: Not in a CRUST repository"));
    }

    // Build path to object
    let object_path = format!(".crust/objects/{}/{}", &id[0..2], &id[2..]);

    if !Path::new(&object_path).exists() {
        return Err(anyhow!("OBJECT_NOT_FOUND: Object {} not found", id));
    }

    // Read and decompress
    let compressed = fs::read(&object_path)
        .map_err(|e| anyhow!("OBJECT_CORRUPT: Failed to read object: {}", e))?;
    let decompressed = zstd::decode_all(&compressed[..])
        .map_err(|e| anyhow!("OBJECT_CORRUPT: Failed to decompress object: {}", e))?;

    if show_type || show_size {
        // Parse header to extract type / size
        let header_end = decompressed
            .windows(2)
            .position(|w| w == b"\n\n")
            .ok_or_else(|| anyhow!("OBJECT_CORRUPT: Invalid object format (no header end)"))?;
        let header = std::str::from_utf8(&decompressed[..header_end])
            .map_err(|_| anyhow!("OBJECT_CORRUPT: Non-UTF8 header"))?;

        let mut obj_type = "";
        let mut obj_size = "";
        for line in header.lines() {
            if let Some(t) = line.strip_prefix("type: ") {
                obj_type = t.trim();
            } else if let Some(s) = line.strip_prefix("size: ") {
                obj_size = s.trim();
            }
        }

        if show_type {
            println!("{}", obj_type);
        } else {
            println!("{}", obj_size);
        }
    } else {
        // Print full decompressed content (header + raw content)
        let output = String::from_utf8_lossy(&decompressed);
        print!("{}", output);
    }

    Ok(())
}
