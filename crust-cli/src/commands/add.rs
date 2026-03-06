// add command - stage files in the index

use crate::index::{Index, IndexEntry};
use crate::working_tree;
use anyhow::{anyhow, Result};
use std::fs;
use std::io::Write;

pub fn cmd_add(paths: &[String]) -> Result<()> {
    let repo_root = ".";

    // Check if we're in a repo
    if !std::path::Path::new(".crust").exists() {
        return Err(anyhow!("CLI_NO_REPOSITORY: Not in a CRUST repository"));
    }

    // Load existing index
    let mut index = Index::load(repo_root)?;
    let mut total_added = 0;

    // Process each path argument
    for path in paths {
        let files = working_tree::scan_working_tree(repo_root, Some(path))?;

        if files.is_empty() {
            eprintln!("warning: {}: No files found", path);
            continue;
        }

        // Add each file to index
        for file in files {
            // Warn if file still contains conflict markers
            if has_conflict_markers(&file.content) {
                eprintln!(
                    "warning: {}: file still contains conflict markers (<<<<<<< / >>>>>>> )",
                    file.path
                );
            }

            let blob_id = file.blob_id();
            let entry = IndexEntry {
                path: file.path.clone(),
                blob_id: blob_id.clone(),
                size: file.size,
                mtime: file.mtime,
            };

            index.add_entry(entry);

            // Save blob object to disk
            save_blob_object(repo_root, &blob_id, &file.path)?;

            // Truncate blob_id for display (show first 8 chars)
            let blob_short = &blob_id[..8.min(blob_id.len())];
            println!("added {} (blob: {}...)", file.path, blob_short);
            total_added += 1;
        }
    }

    if total_added == 0 {
        return Err(anyhow!("No files staged"));
    }

    // Save index
    index.save(repo_root)?;

    Ok(())
}

/// Save blob object to disk with zstd compression
fn save_blob_object(repo_root: &str, blob_id: &str, file_path: &str) -> Result<()> {
    // Read file content
    let content = fs::read(file_path)?;

    // Create blob object (CRUST-OBJECT format)
    let mut blob_object = Vec::new();
    blob_object.extend_from_slice(b"CRUST-OBJECT\n");
    blob_object.extend_from_slice(b"type: blob\n");
    blob_object.extend_from_slice(format!("size: {}\n\n", content.len()).as_bytes());
    blob_object.extend_from_slice(&content);

    // Create objects directory structure
    let objects_dir = format!("{}/.crust/objects", repo_root);
    fs::create_dir_all(&objects_dir)?;

    let subdir = format!("{}/{}", objects_dir, &blob_id[0..2]);
    fs::create_dir_all(&subdir)?;

    let object_path = format!("{}/{}", subdir, &blob_id[2..]);

    // Compress with zstd
    let mut compressed = Vec::new();
    let mut encoder = zstd::Encoder::new(&mut compressed, 0)?;
    encoder.write_all(&blob_object)?;
    encoder.finish()?;

    // Write compressed object to disk
    fs::write(&object_path, &compressed)?;

    Ok(())
}

/// Check if file content contains unresolved conflict markers
fn has_conflict_markers(content: &[u8]) -> bool {
    if let Ok(text) = std::str::from_utf8(content) {
        for line in text.lines() {
            if line.starts_with("<<<<<<<") || line.starts_with(">>>>>>>") {
                return true;
            }
        }
    }
    false
}
