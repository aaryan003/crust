// status command - show working tree status

use crate::commands::checkout::{load_blob_content, load_tree_entries, load_tree_id_from_commit};
use crate::commands::merge;
use crate::index::Index;
use crate::working_tree;
use anyhow::{anyhow, Result};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

pub fn cmd_status() -> Result<()> {
    let repo_root = ".";

    // Check if we're in a repo
    if !Path::new(".crust").exists() {
        return Err(anyhow!("CLI_NO_REPOSITORY: Not in a CRUST repository"));
    }

    // Load index and current branch
    let index = Index::load(repo_root)?;
    let branch = working_tree::get_current_branch(repo_root)?;

    // Build index map: path → blob_id
    let index_map: HashMap<String, String> = index
        .entries()
        .iter()
        .map(|e| (e.path.clone(), e.blob_id.clone()))
        .collect();

    // Build HEAD map: path → blob_id (empty if no commits yet)
    let head_map: HashMap<String, String> = load_head_file_map(repo_root).unwrap_or_default();

    // Scan working tree files
    let working_files = working_tree::scan_working_tree(repo_root, Some("."))?;
    let working_map: HashMap<String, String> = working_files
        .iter()
        .map(|f| (f.path.clone(), f.blob_id()))
        .collect();

    println!("On branch {}\n", branch);

    // Show merge-in-progress banner if MERGE_HEAD exists
    if merge::is_merge_in_progress(repo_root) {
        println!("You are in the middle of a merge.");
        println!("  (fix conflicts and run \"crust commit\" to conclude merge)");
        println!("  (use \"crust merge --abort\" to abort the merge)");

        // Show conflicted files if MERGE_CONFLICTS exists
        let conflicts_path = format!("{}/.crust/MERGE_CONFLICTS", repo_root);
        if let Ok(content) = fs::read_to_string(&conflicts_path) {
            let conflict_files: Vec<&str> = content.lines().filter(|l| !l.is_empty()).collect();
            if !conflict_files.is_empty() {
                println!();
                println!("Unmerged paths:");
                for f in &conflict_files {
                    println!("  both modified:   {}", f);
                }
            }
        }
        println!();
    }

    // ── Staged changes: index vs HEAD ──────────────────────────────────────
    let mut staged: Vec<(String, &str)> = Vec::new();
    for (path, index_blob) in &index_map {
        match head_map.get(path) {
            None => staged.push((path.clone(), "new file")),
            Some(head_blob) if head_blob != index_blob => {
                staged.push((path.clone(), "modified"))
            }
            _ => {} // same as HEAD → will show as nothing staged for this file
        }
    }
    // Deleted from index (in HEAD but not in index) — show as deleted staged
    for path in head_map.keys() {
        if !index_map.contains_key(path) {
            // Only flag as staged-delete if the file is also missing from working tree
            // (otherwise it shows as unstaged modification)
            if !working_map.contains_key(path) {
                staged.push((path.clone(), "deleted"));
            }
        }
    }
    staged.sort_by(|a, b| a.0.cmp(&b.0));

    if !staged.is_empty() {
        println!("Changes staged for commit:");
        for (path, status) in &staged {
            println!("  {:12} {}", status.to_string() + ":", path);
        }
        println!();
    }

    // ── Unstaged changes: working tree vs index (or HEAD if not indexed) ──
    let mut unstaged: Vec<(String, &str)> = Vec::new();

    // Files in index: compare working tree vs index
    for (path, index_blob) in &index_map {
        match working_map.get(path) {
            None => unstaged.push((path.clone(), "deleted")),
            Some(wt_blob) if wt_blob != index_blob => {
                unstaged.push((path.clone(), "modified"))
            }
            _ => {}
        }
    }

    // Files in HEAD but NOT in index: compare working tree vs HEAD
    for (path, head_blob) in &head_map {
        if !index_map.contains_key(path) {
            match working_map.get(path) {
                None => unstaged.push((path.clone(), "deleted")),
                Some(wt_blob) if wt_blob != head_blob => {
                    unstaged.push((path.clone(), "modified"))
                }
                _ => {}
            }
        }
    }
    unstaged.sort_by(|a, b| a.0.cmp(&b.0));

    if !unstaged.is_empty() {
        println!("Changes not staged for commit:");
        for (path, status) in &unstaged {
            println!("  {:12} {}", status.to_string() + ":", path);
        }
        println!();
    }

    // ── Untracked files ────────────────────────────────────────────────────
    let mut untracked: Vec<String> = working_map
        .keys()
        .filter(|p| !index_map.contains_key(*p) && !head_map.contains_key(*p))
        .cloned()
        .collect();
    untracked.sort();

    if !untracked.is_empty() {
        println!("Untracked files:");
        for path in &untracked {
            println!("  {}", path);
        }
        println!();
    }

    if staged.is_empty() && unstaged.is_empty() && untracked.is_empty() {
        println!("nothing to commit, working tree clean");
    }

    Ok(())
}

/// Load a map of path → blob_id from the current HEAD commit.
/// Returns empty map if no commits yet or HEAD can't be read.
fn load_head_file_map(repo_root: &str) -> Result<HashMap<String, String>> {
    let head_ref = working_tree::get_head_ref(repo_root)?;
    let commit_id = match working_tree::read_ref(repo_root, &head_ref)? {
        Some(id) => id,
        None => return Ok(HashMap::new()),
    };
    let tree_id = load_tree_id_from_commit(repo_root, &commit_id)?;
    let entries = load_tree_entries(repo_root, &tree_id)?;
    Ok(entries.into_iter().collect())
}

/// Compute the blob_id for a working-tree file (same formula used in `crust add`).
/// Reused from working_tree::WorkingTreeFile::blob_id() but for the standalone case.
#[allow(dead_code)]
fn compute_blob_id(content: &[u8]) -> String {
    let mut object_bytes = Vec::new();
    object_bytes.extend_from_slice(b"CRUST-OBJECT\n");
    object_bytes.extend_from_slice(b"type: blob\n");
    object_bytes.extend_from_slice(format!("size: {}\n\n", content.len()).as_bytes());
    object_bytes.extend_from_slice(content);
    let mut h = Sha256::new();
    h.update(&object_bytes);
    format!("{:x}", h.finalize())
}

// Keep the old helpers for backwards compatibility (load_blob_content now from checkout)
#[allow(dead_code)]
fn load_blob_content_legacy(repo_root: &str, blob_id: &str) -> Result<Vec<u8>> {
    load_blob_content(repo_root, blob_id)
}

#[allow(dead_code)]
fn parse_tree_for_files(data: &[u8]) -> Result<Vec<String>> {
    let mut files = Vec::new();
    let header_end = data
        .windows(2)
        .position(|w| w == b"\n\n")
        .map(|p| p + 2)
        .unwrap_or(0);
    let content = &data[header_end..];
    let mut pos = 0;
    while pos < content.len() {
        let mut null_pos = pos;
        while null_pos < content.len() && content[null_pos] != 0 {
            null_pos += 1;
        }
        if null_pos >= content.len() { break; }
        if let Ok(mode_name) = std::str::from_utf8(&content[pos..null_pos]) {
            if let Some(sp) = mode_name.find(' ') {
                files.push(mode_name[sp + 1..].to_string());
            }
        }
        pos = null_pos + 1 + 32;
    }
    Ok(files)
}

// Suppress unused import warnings while keeping fs in scope for legacy helpers
#[allow(unused_imports)]
use std::fs as _fs;

