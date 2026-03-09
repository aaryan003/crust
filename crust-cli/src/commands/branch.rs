// branch command - list, create, and delete branches

use crate::refs;
use anyhow::{anyhow, Result};
use std::path::Path;

/// List, create, or delete branches
pub fn cmd_branch(subcommand: Option<&str>, branch_name: Option<&str>, delete: bool, force_delete: bool, verbose: bool) -> Result<()> {
    let repo_root = ".";

    // Check if we're in a repo
    if !Path::new(".crust").exists() {
        return Err(anyhow!("CLI_NO_REPOSITORY: Not in a CRUST repository"));
    }

    if delete || force_delete {
        // Delete branch
        if let Some(name) = branch_name {
            delete_branch(repo_root, name, force_delete)?;
        } else {
            return Err(anyhow!("Branch name required for -d/-D"));
        }
    } else if let Some(name) = subcommand {
        // Create branch at current HEAD
        create_branch(repo_root, name)?;
    } else {
        // List branches
        list_branches(repo_root, verbose)?;
    }

    Ok(())
}

/// List all branches with current branch marked with *
fn list_branches(repo_root: &str, verbose: bool) -> Result<()> {
    let current = refs::get_current_branch(repo_root)?;
    let branches = refs::list_branches(repo_root)?;

    for branch in branches {
        let marker = if branch == current { '*' } else { ' ' };
        if verbose {
            let commit_id = crate::working_tree::read_ref(repo_root, &format!("refs/heads/{}", branch))
                .unwrap_or(None)
                .unwrap_or_default();
            let short_id = if commit_id.len() >= 7 { &commit_id[..7] } else { &commit_id };
            // Try to read commit message first line
            let message = if !commit_id.is_empty() {
                load_commit_first_line(repo_root, &commit_id).unwrap_or_default()
            } else {
                String::new()
            };
            println!("{} {:20} {} {}", marker, branch, short_id, message);
        } else {
            println!("{} {}", marker, branch);
        }
    }

    Ok(())
}

fn load_commit_first_line(repo_root: &str, commit_id: &str) -> Result<String> {
    if commit_id.len() < 3 { return Ok(String::new()); }
    let path = format!("{}/.crust/objects/{}/{}", repo_root, &commit_id[0..2], &commit_id[2..]);
    let compressed = std::fs::read(&path)?;
    let data = zstd::decode_all(&compressed[..])?;
    // Strip CRUST-OBJECT header: find \n\n separator
    if let Some(pos) = data.windows(2).position(|w| w == b"\n\n") {
        let content = &data[pos + 2..];
        let text = std::str::from_utf8(content).unwrap_or("");
        // Find blank line that separates headers from message
        let mut in_message = false;
        for line in text.lines() {
            if in_message {
                return Ok(line.to_string());
            }
            if line.is_empty() {
                in_message = true;
            }
        }
    }
    Ok(String::new())
}

/// Create a new branch at current HEAD
fn create_branch(repo_root: &str, branch_name: &str) -> Result<()> {
    // Get current HEAD commit
    let current_branch = refs::get_current_branch(repo_root)?;
    let head_ref = format!("refs/heads/{}", current_branch);

    let commit_id = match crate::working_tree::read_ref(repo_root, &head_ref)? {
        Some(id) => id,
        None => return Err(anyhow!("No commits in current branch")),
    };

    // Create new branch pointing to same commit
    refs::create_branch(repo_root, branch_name, &commit_id)?;
    println!("Created branch {}", branch_name);

    Ok(())
}

/// Delete a branch (force=true skips unmerged check)
fn delete_branch(repo_root: &str, branch_name: &str, force: bool) -> Result<()> {
    // Refuse to delete current branch
    let current = refs::get_current_branch(repo_root)?;
    if branch_name == current {
        return Err(anyhow!(
            "Cannot delete current branch '{}'. Switch to another branch first.",
            current
        ));
    }

    // Check if branch is merged into current (unless force)
    if !force {
        let current_sha = crate::working_tree::read_ref(repo_root, &format!("refs/heads/{}", current))
            .unwrap_or(None)
            .unwrap_or_default();
        let branch_sha = crate::working_tree::read_ref(repo_root, &format!("refs/heads/{}", branch_name))
            .unwrap_or(None)
            .unwrap_or_default();
        // Simple check: if branch tip is the same as current or is an ancestor of current, it's merged
        // For now, just warn if branch SHA differs from current and neither is empty
        if !current_sha.is_empty() && !branch_sha.is_empty() && current_sha != branch_sha {
            // Check if branch_sha is ancestor of current_sha
            if !is_ancestor_of(repo_root, &branch_sha, &current_sha) {
                return Err(anyhow!(
                    "The branch '{}' is not fully merged. Use -D to force delete.",
                    branch_name
                ));
            }
        }
    }

    refs::delete_branch(repo_root, branch_name)?;
    println!("Deleted branch {}", branch_name);
    Ok(())
}

/// Check if `ancestor_sha` is an ancestor of (or equal to) `descendant_sha`
fn is_ancestor_of(repo_root: &str, ancestor_sha: &str, descendant_sha: &str) -> bool {
    if ancestor_sha == descendant_sha {
        return true;
    }
    let mut queue = std::collections::VecDeque::new();
    let mut visited = std::collections::HashSet::new();
    queue.push_back(descendant_sha.to_string());
    while let Some(current) = queue.pop_front() {
        if visited.contains(&current) { continue; }
        visited.insert(current.clone());
        if current == ancestor_sha { return true; }
        // Load commit parents
        let path = format!("{}/.crust/objects/{}/{}", repo_root, &current[0..2], &current[2..]);
        if let Ok(compressed) = std::fs::read(&path) {
            if let Ok(data) = zstd::decode_all(&compressed[..]) {
                if let Ok(text) = std::str::from_utf8(&data) {
                    for line in text.lines() {
                        if let Some(p) = line.strip_prefix("parent ") {
                            queue.push_back(p.trim().to_string());
                        }
                    }
                }
            }
        }
    }
    false
}