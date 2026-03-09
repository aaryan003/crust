// checkout command - switch branches and update working tree

use crate::{index, refs, working_tree};
use anyhow::{anyhow, Result};
use std::fs;
use std::path::Path;

/// Switch to a branch or create and switch (-b flag), or detach HEAD to a commit SHA
pub fn cmd_checkout(branch_name: &str, create_branch: bool) -> Result<()> {
    let repo_root = ".";

    // Check if we're in a repo
    if !Path::new(".crust").exists() {
        return Err(anyhow!("CLI_NO_REPOSITORY: Not in a CRUST repository"));
    }

    // Check for uncommitted changes (index differs from HEAD, or working tree differs from index)
    let index_obj = index::Index::load(repo_root)?;
    let working = working_tree::scan_working_tree(repo_root, Some("."))?;
    // Allow checkout if index matches HEAD AND working tree matches index
    if has_uncommitted_changes(repo_root, &index_obj, &working)? {
        return Err(anyhow!(
            "CLI_WORKING_TREE_DIRTY: Cannot switch branches with staged changes. Commit or restore first."
        ));
    }
    // Also check if index has staged changes vs HEAD
    if has_staged_changes(repo_root, &index_obj)? {
        return Err(anyhow!(
            "CLI_WORKING_TREE_DIRTY: Cannot switch branches with staged changes. Commit or restore first."
        ));
    }

    if create_branch {
        // Create branch first
        let current_branch = refs::get_current_branch(repo_root)?;
        let head_ref = format!("refs/heads/{}", current_branch);

        let commit_id = match working_tree::read_ref(repo_root, &head_ref)? {
            Some(id) => id,
            None => return Err(anyhow!("No commits in current branch")),
        };

        refs::create_branch(repo_root, branch_name, &commit_id)?;
        println!("Switched to new branch {}", branch_name);

        // Update HEAD to point to new branch
        refs::switch_branch(repo_root, branch_name)?;

        // Restore working tree
        let new_head_ref = format!("refs/heads/{}", branch_name);
        match working_tree::read_ref(repo_root, &new_head_ref)? {
            Some(commit_id) => {
                restore_working_tree(repo_root, &commit_id)?;
                println!("Updated working tree to branch {}", branch_name);
            }
            None => {
                clear_working_tree(repo_root)?;
                println!("New branch has no commits yet");
            }
        }

        return Ok(());
    }

    // Try as branch name first
    let branch_path = format!("{}/.crust/refs/heads/{}", repo_root, branch_name);
    if Path::new(&branch_path).exists() {
        println!("Switched to branch {}", branch_name);

        // Update HEAD to point to new branch
        refs::switch_branch(repo_root, branch_name)?;

        // Restore working tree
        let new_head_ref = format!("refs/heads/{}", branch_name);
        match working_tree::read_ref(repo_root, &new_head_ref)? {
            Some(commit_id) => {
                restore_working_tree(repo_root, &commit_id)?;
                println!("Updated working tree to branch {}", branch_name);
            }
            None => {
                clear_working_tree(repo_root)?;
                println!("New branch has no commits yet");
            }
        }

        return Ok(());
    }

    // Try as commit SHA (full 64-char or short prefix)
    if let Some(full_sha) = resolve_commit_sha(repo_root, branch_name) {
        // Detached HEAD: write the SHA directly into HEAD
        fs::write(
            format!("{}/.crust/HEAD", repo_root),
            format!("{}\n", full_sha),
        )?;
        restore_working_tree(repo_root, &full_sha)?;
        println!("HEAD is now at {} (detached HEAD state)", &full_sha[..12]);
        println!("Updated working tree.");
        return Ok(());
    }

    Err(anyhow!("Branch or commit '{}' not found", branch_name))
}

/// Resolve a short or full SHA prefix to a full 64-char SHA
fn resolve_commit_sha(repo_root: &str, prefix: &str) -> Option<String> {
    // Full SHA — verify it exists
    if prefix.len() == 64 && prefix.chars().all(|c| c.is_ascii_hexdigit()) {
        let path = format!("{}/.crust/objects/{}/{}", repo_root, &prefix[0..2], &prefix[2..]);
        if Path::new(&path).exists() {
            return Some(prefix.to_string());
        }
    }
    // Short SHA prefix (>= 4 chars) — scan objects directory
    if prefix.len() >= 4 && prefix.chars().all(|c| c.is_ascii_hexdigit()) {
        let objects_dir = format!("{}/.crust/objects", repo_root);
        let prefix_dir = &prefix[0..2];
        let rest_prefix = &prefix[2..];
        let dir = format!("{}/{}", objects_dir, prefix_dir);
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                if name_str.starts_with(rest_prefix) {
                    return Some(format!("{}{}", prefix_dir, name_str));
                }
            }
        }
    }
    None
}

/// Public alias used by merge.rs for fast-forward merges.
pub fn restore_working_tree_pub(repo_root: &str, commit_id: &str) -> Result<()> {
    restore_working_tree(repo_root, commit_id)
}

/// Checkout specific files from another branch without switching branches.
/// Equivalent to `git checkout <branch> -- <files>`
pub fn cmd_checkout_files(branch_name: &str, files: &[String]) -> Result<()> {
    let repo_root = ".";
    if !Path::new(".crust").exists() {
        return Err(anyhow!("CLI_NO_REPOSITORY: Not in a CRUST repository"));
    }

    // Resolve branch to commit
    let commit_id = match working_tree::read_ref(repo_root, &format!("refs/heads/{}", branch_name))? {
        Some(id) => id,
        None => return Err(anyhow!("Branch '{}' not found", branch_name)),
    };

    let tree_id = load_tree_id_from_commit(repo_root, &commit_id)?;
    let entries: std::collections::HashMap<_, _> =
        load_tree_entries(repo_root, &tree_id)?.into_iter().collect();

    for file in files {
        match entries.get(file.as_str()) {
            Some(blob_id) => {
                let content = load_blob_content(repo_root, blob_id)?;
                if let Some(parent) = Path::new(file).parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::write(file, content)?;
                println!("Restored '{}' from branch '{}'", file, branch_name);
            }
            None => {
                eprintln!("Warning: file '{}' not found in branch '{}'", file, branch_name);
            }
        }
    }
    Ok(())
}

/// Restore all working tree files to the state of a given commit.
/// Reads the commit → tree → blobs and writes each file.
fn restore_working_tree(repo_root: &str, commit_id: &str) -> Result<()> {
    // Load commit → get tree ID
    let tree_id = load_tree_id_from_commit(repo_root, commit_id)?;

    // Load tree → get (path, blob_id) entries
    let entries = load_tree_entries(repo_root, &tree_id)?;

    // Collect current working-tree file paths (to detect deletions)
    let current_files: Vec<_> = working_tree::scan_working_tree(repo_root, Some("."))?
        .into_iter()
        .map(|f| f.path)
        .collect();

    // Write each file from blob storage
    for (file_path, blob_id) in &entries {
        let content = load_blob_content(repo_root, blob_id)?;
        let full_path = format!("{}/{}", repo_root, file_path);
        if let Some(parent) = Path::new(&full_path).parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&full_path, &content)?;
    }

    // Remove files that exist in the working tree but are NOT in the target tree
    let target_paths: std::collections::HashSet<String> =
        entries.iter().map(|(p, _)| p.clone()).collect();
    for path in &current_files {
        if !target_paths.contains(path) {
            let full_path = format!("{}/{}", repo_root, path);
            let _ = fs::remove_file(&full_path);
        }
    }

    // Populate the index with the checked-out tree's files
    // (so the working tree matches HEAD and the index mirrors it)
    let mut new_index = index::Index::new();
    for (file_path, blob_id) in &entries {
        new_index.add_entry(index::IndexEntry {
            path: file_path.clone(),
            blob_id: blob_id.clone(),
            size: 0,
            mtime: 0,
        });
    }
    new_index.save(repo_root)?;

    Ok(())
}

/// Remove all tracked working-tree files (switching to empty branch)
fn clear_working_tree(repo_root: &str) -> Result<()> {
    let files = working_tree::scan_working_tree(repo_root, Some("."))?;
    for file in files {
        let full_path = format!("{}/{}", repo_root, file.path);
        let _ = fs::remove_file(&full_path);
    }
    let empty_index = index::Index::new();
    empty_index.save(repo_root)?;
    Ok(())
}

/// Read a zstd-compressed CRUST object, decompress, return raw bytes after header
pub fn load_blob_content(repo_root: &str, blob_id: &str) -> Result<Vec<u8>> {
    let object_path = format!(
        "{}/.crust/objects/{}/{}",
        repo_root,
        &blob_id[0..2],
        &blob_id[2..]
    );
    if !Path::new(&object_path).exists() {
        return Err(anyhow!("Blob object not found: {}", blob_id));
    }
    let compressed = fs::read(&object_path)?;
    let data = zstd::decode_all(&compressed[..])?;
    // Strip "CRUST-OBJECT\ntype: blob\nsize: N\n\n" header
    strip_object_header(&data)
}

/// Strip the CRUST-OBJECT header and return just the content bytes
pub fn strip_object_header(data: &[u8]) -> Result<Vec<u8>> {
    // Find the double-newline separating header from content
    for i in 0..data.len().saturating_sub(1) {
        if data[i] == b'\n' && data[i + 1] == b'\n' {
            return Ok(data[i + 2..].to_vec());
        }
    }
    Err(anyhow!("Invalid CRUST object: no header separator found"))
}

/// Load the tree ID from a commit object on disk
pub fn load_tree_id_from_commit(repo_root: &str, commit_id: &str) -> Result<String> {
    let object_path = format!(
        "{}/.crust/objects/{}/{}",
        repo_root,
        &commit_id[0..2],
        &commit_id[2..]
    );
    if !Path::new(&object_path).exists() {
        return Err(anyhow!("Commit object not found: {}", commit_id));
    }
    let compressed = fs::read(&object_path)?;
    let data = zstd::decode_all(&compressed[..])?;
    let content = strip_object_header(&data)?;
    let text = std::str::from_utf8(&content)?;
    for line in text.lines() {
        if let Some(tree_id) = line.strip_prefix("tree ") {
            return Ok(tree_id.trim().to_string());
        }
    }
    Err(anyhow!("No tree ID found in commit {}", commit_id))
}

/// Load all (path, blob_id) entries from a tree object on disk
pub fn load_tree_entries(repo_root: &str, tree_id: &str) -> Result<Vec<(String, String)>> {
    let object_path = format!(
        "{}/.crust/objects/{}/{}",
        repo_root,
        &tree_id[0..2],
        &tree_id[2..]
    );
    if !Path::new(&object_path).exists() {
        return Err(anyhow!("Tree object not found: {}", tree_id));
    }
    let compressed = fs::read(&object_path)?;
    let data = zstd::decode_all(&compressed[..])?;

    // Find header/content split
    let mut header_end = 0;
    for i in 0..data.len().saturating_sub(1) {
        if data[i] == b'\n' && data[i + 1] == b'\n' {
            header_end = i + 2;
            break;
        }
    }
    let content = &data[header_end..];

    // Parse binary tree entries: "100644 {name}\0{32-byte sha}"
    let mut entries = Vec::new();
    let mut pos = 0;
    while pos < content.len() {
        let mut null_pos = pos;
        while null_pos < content.len() && content[null_pos] != 0 {
            null_pos += 1;
        }
        if null_pos + 1 + 32 > content.len() {
            break;
        }
        if let Ok(mode_name) = std::str::from_utf8(&content[pos..null_pos]) {
            if let Some(space_pos) = mode_name.find(' ') {
                let filename = mode_name[space_pos + 1..].to_string();
                let sha_bytes = &content[null_pos + 1..null_pos + 1 + 32];
                let blob_id = hex::encode(sha_bytes);
                entries.push((filename, blob_id));
            }
        }
        pos = null_pos + 1 + 32;
    }
    Ok(entries)
}

/// Check if the index has staged changes vs HEAD commit (new files or modified files)
fn has_staged_changes(repo_root: &str, index_obj: &index::Index) -> Result<bool> {
    // Get HEAD commit tree
    let head_ref = match working_tree::get_head_ref(repo_root) {
        Ok(r) => r,
        Err(_) => return Ok(false), // No HEAD yet
    };
    let commit_id = match working_tree::read_ref(repo_root, &head_ref)? {
        Some(id) => id,
        None => {
            // No commits yet — if index is non-empty, that's a staged change
            return Ok(!index_obj.is_empty());
        }
    };
    let tree_id = match load_tree_id_from_commit(repo_root, &commit_id) {
        Ok(t) => t,
        Err(_) => return Ok(!index_obj.is_empty()),
    };
    let head_entries: std::collections::HashMap<String, String> =
        load_tree_entries(repo_root, &tree_id)?.into_iter().collect();

    // Compare index vs HEAD
    let index_map: std::collections::HashMap<String, String> = index_obj
        .entries
        .iter()
        .map(|e| (e.path.clone(), e.blob_id.clone()))
        .collect();

    // Any file in index not in HEAD or with different blob_id?
    for (path, blob_id) in &index_map {
        match head_entries.get(path) {
            None => return Ok(true),            // New file staged
            Some(head_blob) if head_blob != blob_id => return Ok(true), // Modified staged
            _ => {}
        }
    }

    // Any file in HEAD not in index (staged deletion)?
    for path in head_entries.keys() {
        if !index_map.contains_key(path) {
            return Ok(true);
        }
    }

    Ok(false)
}

/// Check if there are uncommitted changes (working tree vs index)
fn has_uncommitted_changes(
    _repo_root: &str,
    index_obj: &index::Index,
    working: &[working_tree::WorkingTreeFile],
) -> Result<bool> {
    // Check if any working tree files differ from index
    let index_map: std::collections::HashMap<String, String> = index_obj
        .entries
        .iter()
        .map(|e| (e.path.clone(), e.blob_id.clone()))
        .collect();

    for file in working {
        let blob_id = file.blob_id();
        if let Some(indexed_id) = index_map.get(&file.path) {
            if blob_id != *indexed_id {
                return Ok(true); // File modified
            }
        } else {
            return Ok(true); // File not in index (untracked or newly modified)
        }
    }

    // Check if any indexed files were deleted from working tree
    let working_paths: std::collections::HashSet<String> =
        working.iter().map(|f| f.path.clone()).collect();

    for entry in &index_obj.entries {
        if !working_paths.contains(&entry.path) {
            return Ok(true); // File was deleted
        }
    }

    Ok(false)
}
