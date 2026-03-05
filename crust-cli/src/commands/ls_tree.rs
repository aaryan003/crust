// ls-tree command - list tree entries

use crate::commands::checkout::{load_tree_entries, load_tree_id_from_commit};
use crate::working_tree;
use anyhow::{anyhow, Result};
use std::collections::VecDeque;
use std::fs;
use std::path::Path;

/// List tree entries in a tree object.
/// `id` can be: a 64-char SHA256, a branch name, or "HEAD".
/// `recursive` lists all blobs recursively (-r).
/// `name_only` suppresses metadata, shows only file paths.
/// `prefix_filter` restricts output to a subdirectory (e.g. "src/").
pub fn cmd_ls_tree(id: &str, recursive: bool, name_only: bool, prefix_filter: Option<&str>) -> Result<()> {
    if !Path::new(".crust").exists() {
        return Err(anyhow!("CLI_NO_REPOSITORY: Not in a CRUST repository"));
    }
    let repo_root = ".";
    let tree_id = resolve_to_tree(repo_root, id)?;
    list_tree(repo_root, &tree_id, "", recursive, name_only, prefix_filter)?;
    Ok(())
}

fn list_tree(
    repo_root: &str,
    tree_id: &str,
    prefix: &str,
    recursive: bool,
    name_only: bool,
    prefix_filter: Option<&str>,
) -> Result<()> {
    let entries = load_tree_entries(repo_root, tree_id)?;
    if entries.is_empty() && prefix.is_empty() {
        println!("(empty tree)");
        return Ok(());
    }
    for (name, blob_id) in &entries {
        let full_path = if prefix.is_empty() {
            name.clone()
        } else {
            format!("{}/{}", prefix, name)
        };

        // Determine type by looking at the blob_id object type
        let obj_type = get_object_type(repo_root, blob_id).unwrap_or_else(|_| "blob".to_string());

        if obj_type == "tree" {
            if recursive {
                list_tree(repo_root, blob_id, &full_path, true, name_only, prefix_filter)?;
            } else {
                // Show tree entry if it matches prefix filter
                if matches_filter(&full_path, prefix_filter) {
                    if name_only {
                        println!("{}/", full_path);
                    } else {
                        println!("040000 tree {}    {}/", blob_id, full_path);
                    }
                }
            }
        } else {
            if matches_filter(&full_path, prefix_filter) {
                if name_only {
                    println!("{}", full_path);
                } else {
                    println!("100644 blob {}    {}", blob_id, full_path);
                }
            }
        }
    }
    Ok(())
}

fn matches_filter(path: &str, filter: Option<&str>) -> bool {
    match filter {
        None => true,
        Some(f) => {
            let f = f.trim_end_matches('/');
            path.starts_with(f)
        }
    }
}

fn get_object_type(repo_root: &str, object_id: &str) -> Result<String> {
    if object_id.len() < 3 {
        return Ok("blob".to_string());
    }
    let path = format!(
        "{}/.crust/objects/{}/{}",
        repo_root,
        &object_id[0..2],
        &object_id[2..]
    );
    if !Path::new(&path).exists() {
        return Ok("blob".to_string());
    }
    let compressed = fs::read(&path)?;
    let data = zstd::decode_all(&compressed[..])?;
    let text = std::str::from_utf8(&data).unwrap_or("");
    for line in text.lines().take(3) {
        if let Some(t) = line.strip_prefix("type: ") {
            return Ok(t.to_string());
        }
    }
    Ok("blob".to_string())
}

/// Resolve an id (SHA, branch name, or HEAD) to a tree SHA.
fn resolve_to_tree(repo_root: &str, id: &str) -> Result<String> {
    // If it looks like a commit or tree SHA (64 hex chars), figure out which
    if id.len() == 64 && id.chars().all(|c| c.is_ascii_hexdigit()) {
        let object_path = format!("{}/.crust/objects/{}/{}", repo_root, &id[0..2], &id[2..]);
        if !Path::new(&object_path).exists() {
            return Err(anyhow!("OBJECT_NOT_FOUND: Object {} not found", id));
        }
        let compressed = fs::read(&object_path)?;
        let data = zstd::decode_all(&compressed[..])?;
        let text = std::str::from_utf8(&data)?;
        if let Some(line) = text.lines().nth(1) {
            if line == "type: tree" {
                return Ok(id.to_string()); // it's already a tree
            } else if line == "type: commit" {
                return load_tree_id_from_commit(repo_root, id);
            }
        }
        return Err(anyhow!("Object {} is neither a commit nor a tree", id));
    }

    // Handle "HEAD" or a branch name
    let commit_id = if id == "HEAD" {
        let head_ref = working_tree::get_head_ref(repo_root)?;
        match working_tree::read_ref(repo_root, &head_ref)? {
            Some(cid) => cid,
            None => return Err(anyhow!("HEAD has no commits yet")),
        }
    } else {
        // Try as branch name
        match working_tree::read_ref(repo_root, &format!("refs/heads/{}", id))? {
            Some(cid) => cid,
            None => return Err(anyhow!("Ref not found: {}", id)),
        }
    };

    load_tree_id_from_commit(repo_root, &commit_id)
}
