// pull command - fetch and merge/rebase

use crate::commands::cmd_fetch;
use crate::commands::cmd_merge;
use crate::refs;
use crate::working_tree;
use anyhow::{anyhow, Result};
use sha2::{Digest, Sha256};
use std::collections::{HashSet, VecDeque};
use std::fs;
use std::path::Path;

pub fn cmd_pull(remote: Option<&str>, branch: Option<&str>, rebase: bool) -> Result<()> {
    // Fetch from remote
    cmd_fetch(remote, branch)?;

    let remote_name = remote.unwrap_or("origin");
    let branch_name = if let Some(b) = branch {
        b.to_string()
    } else if Path::new(".crust").exists() {
        refs::get_current_branch(".").unwrap_or_else(|_| "main".to_string())
    } else {
        "main".to_string()
    };

    if rebase {
        cmd_rebase_onto_remote(".", remote_name, &branch_name)
    } else {
        let merge_ref = format!("remotes/{}/{}", remote_name, branch_name);
        println!("Merging {}...", merge_ref);
        cmd_merge(&merge_ref)
    }
}

/// Rebase current branch onto the remote tracking branch tip.
/// Collects local commits since merge-base, resets to remote tip, replays them.
fn cmd_rebase_onto_remote(repo_root: &str, remote_name: &str, branch_name: &str) -> Result<()> {
    let current_branch = refs::get_current_branch(repo_root)?;

    let local_tip = match working_tree::read_ref(repo_root, &format!("refs/heads/{}", current_branch))? {
        Some(id) => id,
        None => return Err(anyhow!("No commits on current branch '{}'", current_branch)),
    };

    // Remote tracking ref
    let remote_tracking = format!("refs/remotes/{}/{}", remote_name, branch_name);
    let remote_tip = match working_tree::read_ref(repo_root, &remote_tracking)? {
        Some(id) => id,
        None => {
            // Nothing on remote — nothing to rebase onto
            println!("Already up to date.");
            return Ok(());
        }
    };

    if local_tip == remote_tip {
        println!("Already up to date.");
        return Ok(());
    }

    // If local_tip is already an ancestor of remote_tip, fast-forward
    if is_ancestor(repo_root, &local_tip, &remote_tip)? {
        refs::update_branch(repo_root, &current_branch, &remote_tip)?;
        crate::commands::checkout::restore_working_tree_pub(repo_root, &remote_tip)?;
        println!("Fast-forward (rebase)");
        println!("Updated {} -> {}", &local_tip[..7], &remote_tip[..7]);
        return Ok(());
    }

    // Find merge-base
    let base = find_merge_base(repo_root, &local_tip, &remote_tip);

    // Collect commits from local_tip back to base (exclusive), oldest-first
    let local_commits = collect_commits_since(repo_root, &local_tip, &base)?;

    if local_commits.is_empty() {
        // No local commits to replay — just fast-forward
        refs::update_branch(repo_root, &current_branch, &remote_tip)?;
        crate::commands::checkout::restore_working_tree_pub(repo_root, &remote_tip)?;
        println!("Already up to date.");
        return Ok(());
    }

    println!("Rebasing {} commit(s) onto {}...", local_commits.len(), &remote_tip[..7]);

    // Replay each commit on top of remote_tip
    let mut new_parent = remote_tip.clone();
    for commit_id in &local_commits {
        let (tree_id, author, committer, message) = load_commit_fields(repo_root, commit_id)?;
        new_parent = create_rebased_commit(repo_root, &tree_id, &new_parent, &author, &committer, &message)?;
        println!("  {} -> {}", &commit_id[..7], &new_parent[..7]);
    }

    // Update branch ref and restore working tree
    refs::update_branch(repo_root, &current_branch, &new_parent)?;
    crate::commands::checkout::restore_working_tree_pub(repo_root, &new_parent)?;

    println!("Successfully rebased {} onto {}", current_branch, &remote_tip[..7]);
    Ok(())
}

/// Collect commits from `tip` back to (but not including) `base`, oldest first.
fn collect_commits_since(repo_root: &str, tip: &str, base: &str) -> Result<Vec<String>> {
    let mut result = Vec::new();
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();
    queue.push_back(tip.to_string());

    while let Some(id) = queue.pop_front() {
        if visited.contains(&id) || id == base {
            continue;
        }
        visited.insert(id.clone());
        result.push(id.clone());
        for parent in load_commit_parents(repo_root, &id) {
            queue.push_back(parent);
        }
    }
    // Reverse so oldest is first
    result.reverse();
    Ok(result)
}

/// Parse commit object, return (tree_id, author, committer, message)
fn load_commit_fields(repo_root: &str, commit_id: &str) -> Result<(String, String, String, String)> {
    let object_path = format!(
        "{}/.crust/objects/{}/{}",
        repo_root,
        &commit_id[0..2],
        &commit_id[2..]
    );
    let compressed = fs::read(&object_path)?;
    let data = zstd::decode_all(&compressed[..])?;

    // Strip CRUST-OBJECT header (find first \n\n)
    let sep = data.windows(2).position(|w| w == b"\n\n")
        .ok_or_else(|| anyhow!("Malformed commit object {}", commit_id))?;
    let content = &data[sep + 2..];
    let text = std::str::from_utf8(content)?;

    let mut tree_id = String::new();
    let mut author = String::new();
    let mut committer = String::new();
    let mut message_lines = Vec::new();
    let mut in_message = false;

    for line in text.lines() {
        if in_message {
            message_lines.push(line);
        } else if line.is_empty() {
            in_message = true;
        } else if let Some(v) = line.strip_prefix("tree ") {
            tree_id = v.trim().to_string();
        } else if let Some(v) = line.strip_prefix("author ") {
            author = v.trim().to_string();
        } else if let Some(v) = line.strip_prefix("committer ") {
            committer = v.trim().to_string();
        }
        // skip parent lines — we're rebasing, so parent changes
    }

    Ok((tree_id, author, committer, message_lines.join("\n")))
}

/// Create a new commit object with a new parent, return the new commit id.
fn create_rebased_commit(
    repo_root: &str,
    tree_id: &str,
    parent_id: &str,
    author: &str,
    committer: &str,
    message: &str,
) -> Result<String> {
    let mut content = Vec::new();
    content.extend_from_slice(format!("tree {}\n", tree_id).as_bytes());
    content.extend_from_slice(format!("parent {}\n", parent_id).as_bytes());
    content.extend_from_slice(format!("author {}\n", author).as_bytes());
    content.extend_from_slice(format!("committer {}\n", committer).as_bytes());
    content.push(b'\n');
    content.extend_from_slice(message.as_bytes());

    let mut object = Vec::new();
    object.extend_from_slice(b"CRUST-OBJECT\n");
    object.extend_from_slice(b"type: commit\n");
    object.extend_from_slice(format!("size: {}\n\n", content.len()).as_bytes());
    object.extend_from_slice(&content);

    let mut hasher = Sha256::new();
    hasher.update(&object);
    let commit_id = format!("{:x}", hasher.finalize());

    save_object(repo_root, &commit_id, &object)?;
    Ok(commit_id)
}

fn save_object(repo_root: &str, object_id: &str, data: &[u8]) -> Result<()> {
    let objects_dir = format!("{}/.crust/objects", repo_root);
    fs::create_dir_all(&objects_dir)?;
    let subdir = format!("{}/{}", objects_dir, &object_id[0..2]);
    fs::create_dir_all(&subdir)?;
    let path = format!("{}/{}", subdir, &object_id[2..]);
    if !Path::new(&path).exists() {
        let mut compressed = Vec::new();
        let mut encoder = zstd::Encoder::new(&mut compressed, 0)?;
        std::io::Write::write_all(&mut encoder, data)?;
        encoder.finish()?;
        let tmp = format!("{}.tmp", path);
        fs::write(&tmp, &compressed)?;
        fs::rename(&tmp, &path)?;
    }
    Ok(())
}

fn is_ancestor(repo_root: &str, ancestor_id: &str, descendant_id: &str) -> Result<bool> {
    let mut queue = VecDeque::new();
    let mut visited = HashSet::new();
    queue.push_back(descendant_id.to_string());
    while let Some(id) = queue.pop_front() {
        if visited.contains(&id) { continue; }
        if id == ancestor_id { return Ok(true); }
        visited.insert(id.clone());
        for p in load_commit_parents(repo_root, &id) {
            queue.push_back(p);
        }
    }
    Ok(false)
}

fn find_merge_base(repo_root: &str, a: &str, b: &str) -> String {
    let mut ancestors_a = HashSet::new();
    let mut queue = VecDeque::new();
    queue.push_back(a.to_string());
    while let Some(id) = queue.pop_front() {
        if ancestors_a.contains(&id) { continue; }
        ancestors_a.insert(id.clone());
        for p in load_commit_parents(repo_root, &id) { queue.push_back(p); }
    }
    let mut queue = VecDeque::new();
    let mut visited = HashSet::new();
    queue.push_back(b.to_string());
    while let Some(id) = queue.pop_front() {
        if visited.contains(&id) { continue; }
        if ancestors_a.contains(&id) { return id; }
        visited.insert(id.clone());
        for p in load_commit_parents(repo_root, &id) { queue.push_back(p); }
    }
    String::new()
}

fn load_commit_parents(repo_root: &str, commit_id: &str) -> Vec<String> {
    let object_path = format!(
        "{}/.crust/objects/{}/{}",
        repo_root,
        &commit_id[0..2],
        &commit_id[2..]
    );
    let Ok(compressed) = fs::read(&object_path) else { return vec![]; };
    let Ok(data) = zstd::decode_all(&compressed[..]) else { return vec![]; };
    let sep = match data.windows(2).position(|w| w == b"\n\n") {
        Some(p) => p,
        None => return vec![],
    };
    let content = &data[sep + 2..];
    let Ok(text) = std::str::from_utf8(content) else { return vec![]; };
    text.lines()
        .filter_map(|l| l.strip_prefix("parent ").map(|p| p.trim().to_string()))
        .collect()
}
