// merge command - merge branches with 3-way merge and conflict detection

use crate::commands::checkout::{load_blob_content, load_tree_entries, load_tree_id_from_commit};
use crate::{index, refs, working_tree};
use anyhow::{anyhow, Result};
use chrono::Utc;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

/// Merge another branch into current branch
pub fn cmd_merge(source_branch: &str) -> Result<()> {
    let repo_root = ".";

    if !Path::new(".crust").exists() {
        return Err(anyhow!("CLI_NO_REPOSITORY: Not in a CRUST repository"));
    }

    let current_branch = refs::get_current_branch(repo_root)?;

    if current_branch == source_branch {
        return Err(anyhow!("Cannot merge branch into itself"));
    }

    let current_commit_id =
        match working_tree::read_ref(repo_root, &format!("refs/heads/{}", current_branch))? {
            Some(id) => id,
            None => {
                return Err(anyhow!(
                    "No commits in current branch '{}'",
                    current_branch
                ))
            }
        };

    let source_commit_id = resolve_ref(repo_root, source_branch)?;

    if current_commit_id == source_commit_id {
        println!("Already up to date.");
        return Ok(());
    }

    // Already merged?
    if is_ancestor(repo_root, &source_commit_id, &current_commit_id)? {
        println!("Already up to date.");
        return Ok(());
    }

    // Fast-forward: current is an ancestor of source
    if is_ancestor(repo_root, &current_commit_id, &source_commit_id)? {
        refs::update_branch(repo_root, &current_branch, &source_commit_id)?;
        crate::commands::checkout::restore_working_tree_pub(repo_root, &source_commit_id)?;
        println!("Fast-forward");
        println!(
            "Updated {} -> {}",
            &current_commit_id[..7],
            &source_commit_id[..7]
        );
        return Ok(());
    }

    // 3-way merge: find common ancestor
    let base_commit_id =
        find_merge_base(repo_root, &current_commit_id, &source_commit_id)?;

    let current_tree_id = load_tree_id_from_commit(repo_root, &current_commit_id)?;
    let source_tree_id = load_tree_id_from_commit(repo_root, &source_commit_id)?;

    let current_files: HashMap<String, String> =
        load_tree_entries(repo_root, &current_tree_id)?.into_iter().collect();
    let source_files: HashMap<String, String> =
        load_tree_entries(repo_root, &source_tree_id)?.into_iter().collect();
    let base_files: HashMap<String, String> = if !base_commit_id.is_empty() {
        if let Ok(btid) = load_tree_id_from_commit(repo_root, &base_commit_id) {
            load_tree_entries(repo_root, &btid)?.into_iter().collect()
        } else {
            HashMap::new()
        }
    } else {
        HashMap::new()
    };

    let all_paths: HashSet<String> = current_files
        .keys()
        .chain(source_files.keys())
        .chain(base_files.keys())
        .cloned()
        .collect();

    let mut conflicts: Vec<String> = Vec::new();
    let mut merged_files: Vec<(String, Vec<u8>)> = Vec::new();

    for path in &all_paths {
        let base_id = base_files.get(path);
        let current_id = current_files.get(path);
        let source_id = source_files.get(path);

        match (base_id, current_id, source_id) {
            // Only in source → take source
            (None, None, Some(sid)) => {
                merged_files.push((path.clone(), load_blob_content(repo_root, sid)?));
            }
            // Only in current → keep current
            (None, Some(cid), None) => {
                merged_files.push((path.clone(), load_blob_content(repo_root, cid)?));
            }
            // Identical on both sides → keep
            (_, Some(cid), Some(sid)) if cid == sid => {
                merged_files.push((path.clone(), load_blob_content(repo_root, cid)?));
            }
            // Only source changed (base == current)
            (Some(bid), Some(cid), Some(sid)) if bid == cid => {
                merged_files.push((path.clone(), load_blob_content(repo_root, sid)?));
            }
            // Only current changed (base == source)
            (Some(bid), Some(cid), Some(sid)) if bid == sid => {
                merged_files.push((path.clone(), load_blob_content(repo_root, cid)?));
            }
            // Both changed differently → conflict
            (_, Some(cid), Some(sid)) if cid != sid => {
                let ours = load_blob_content(repo_root, cid)?;
                let theirs = load_blob_content(repo_root, sid)?;
                let conflict_content =
                    build_conflict_content(&ours, &theirs, &current_branch, source_branch);
                merged_files.push((path.clone(), conflict_content));
                conflicts.push(path.clone());
            }
            // Source deleted it (base == current, source None) → delete
            (Some(bid), Some(cid), None) if bid == cid => { /* delete */ }
            // Current deleted it (base == source, current None) → delete
            (Some(bid), None, Some(sid)) if bid == sid => { /* delete */ }
            // Delete/modify conflict → keep current, flag conflict
            (Some(_), Some(cid), None) => {
                merged_files.push((path.clone(), load_blob_content(repo_root, cid)?));
                conflicts.push(path.clone());
            }
            (Some(_), None, Some(sid)) => {
                merged_files.push((path.clone(), load_blob_content(repo_root, sid)?));
                conflicts.push(path.clone());
            }
            // Fallback: keep current
            _ => {
                if let Some(cid) = current_id {
                    merged_files.push((path.clone(), load_blob_content(repo_root, cid)?));
                }
            }
        }
    }

    // Write merged files to working tree
    for (path, content) in &merged_files {
        let full_path = format!("{}/{}", repo_root, path);
        if let Some(parent) = Path::new(&full_path).parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&full_path, content)?;
    }

    // Remove files deleted by the merge
    let merged_paths: HashSet<String> = merged_files.iter().map(|(p, _)| p.clone()).collect();
    for path in current_files.keys() {
        if !merged_paths.contains(path) {
            let _ = fs::remove_file(format!("{}/{}", repo_root, path));
        }
    }

    if !conflicts.is_empty() {
        println!("CONFLICT (content): Merge conflict in:");
        for c in &conflicts {
            println!("  {}", c);
        }
        println!();
        println!("Automatic merge failed; fix conflicts and then commit the result.");
        println!("  (use \"crust add <file>\" to mark resolved)");
        return Err(anyhow!("MERGE_CONFLICT: {} conflict(s) found", conflicts.len()));
    }

    println!("Merge made by the 3-way strategy.");
    println!(" {} file(s) changed", merged_files.len());

    let merge_commit_id = create_merge_commit(
        repo_root,
        &current_branch,
        &current_commit_id,
        &source_commit_id,
        source_branch,
        &merged_files,
    )?;

    println!("Merge commit: {}", &merge_commit_id[..7]);

    Ok(())
}

fn resolve_ref(repo_root: &str, branch: &str) -> Result<String> {
    if let Some(id) = working_tree::read_ref(repo_root, &format!("refs/heads/{}", branch))? {
        return Ok(id);
    }
    if let Some(id) = working_tree::read_ref(repo_root, &format!("refs/{}", branch))? {
        return Ok(id);
    }
    if let Some(id) = working_tree::read_ref(repo_root, branch)? {
        return Ok(id);
    }
    Err(anyhow!("Branch '{}' not found", branch))
}

fn is_ancestor(repo_root: &str, ancestor_id: &str, descendant_id: &str) -> Result<bool> {
    let mut queue = vec![descendant_id.to_string()];
    let mut visited = HashSet::new();
    while let Some(id) = queue.pop() {
        if visited.contains(&id) {
            continue;
        }
        if id == ancestor_id {
            return Ok(true);
        }
        visited.insert(id.clone());
        if let Ok(parents) = load_commit_parents(repo_root, &id) {
            queue.extend(parents);
        }
    }
    Ok(false)
}

fn find_merge_base(repo_root: &str, a: &str, b: &str) -> Result<String> {
    let mut ancestors_a = HashSet::new();
    let mut queue = vec![a.to_string()];
    while let Some(id) = queue.pop() {
        if ancestors_a.contains(&id) {
            continue;
        }
        ancestors_a.insert(id.clone());
        if let Ok(parents) = load_commit_parents(repo_root, &id) {
            queue.extend(parents);
        }
    }
    let mut queue = vec![b.to_string()];
    let mut visited = HashSet::new();
    while let Some(id) = queue.pop() {
        if visited.contains(&id) {
            continue;
        }
        if ancestors_a.contains(&id) {
            return Ok(id);
        }
        visited.insert(id.clone());
        if let Ok(parents) = load_commit_parents(repo_root, &id) {
            queue.extend(parents);
        }
    }
    Ok(String::new())
}

fn load_commit_parents(repo_root: &str, commit_id: &str) -> Result<Vec<String>> {
    let object_path = format!(
        "{}/.crust/objects/{}/{}",
        repo_root,
        &commit_id[0..2],
        &commit_id[2..]
    );
    if !Path::new(&object_path).exists() {
        return Ok(vec![]);
    }
    let compressed = fs::read(&object_path)?;
    let data = zstd::decode_all(&compressed[..])?;
    let content = crate::commands::checkout::strip_object_header(&data)?;
    let text = std::str::from_utf8(&content)?;
    let mut parents = Vec::new();
    for line in text.lines() {
        if let Some(parent_id) = line.strip_prefix("parent ") {
            parents.push(parent_id.trim().to_string());
        }
    }
    Ok(parents)
}

fn build_conflict_content(
    ours: &[u8],
    theirs: &[u8],
    our_label: &str,
    their_label: &str,
) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(format!("<<<<<<< {}\n", our_label).as_bytes());
    out.extend_from_slice(ours);
    if !ours.ends_with(b"\n") {
        out.push(b'\n');
    }
    out.extend_from_slice(b"=======\n");
    out.extend_from_slice(theirs);
    if !theirs.ends_with(b"\n") {
        out.push(b'\n');
    }
    out.extend_from_slice(format!(">>>>>>> {}\n", their_label).as_bytes());
    out
}

fn create_merge_commit(
    repo_root: &str,
    branch: &str,
    current_id: &str,
    source_id: &str,
    source_branch: &str,
    merged_files: &[(String, Vec<u8>)],
) -> Result<String> {
    let mut tree_content = Vec::new();
    let mut sorted: Vec<_> = merged_files.iter().collect();
    sorted.sort_by(|a, b| a.0.cmp(&b.0));

    for (path, content) in &sorted {
        let mut blob_data = Vec::new();
        blob_data.extend_from_slice(b"CRUST-OBJECT\n");
        blob_data.extend_from_slice(b"type: blob\n");
        blob_data.extend_from_slice(format!("size: {}\n\n", content.len()).as_bytes());
        blob_data.extend_from_slice(content);

        let mut hasher = Sha256::new();
        hasher.update(&blob_data);
        let blob_id = format!("{:x}", hasher.finalize());
        save_object(repo_root, &blob_id, &blob_data)?;

        tree_content.extend_from_slice(b"100644 ");
        tree_content.extend_from_slice(path.as_bytes());
        tree_content.push(0);
        tree_content.extend_from_slice(&hex::decode(&blob_id)?);
    }

    let mut tree_obj = Vec::new();
    tree_obj.extend_from_slice(b"CRUST-OBJECT\n");
    tree_obj.extend_from_slice(b"type: tree\n");
    tree_obj.extend_from_slice(format!("size: {}\n\n", tree_content.len()).as_bytes());
    tree_obj.extend_from_slice(&tree_content);

    let mut hasher = Sha256::new();
    hasher.update(&tree_obj);
    let tree_id = format!("{:x}", hasher.finalize());
    save_object(repo_root, &tree_id, &tree_obj)?;

    let now = Utc::now();
    let timestamp = now.timestamp();
    let mut commit_content = Vec::new();
    commit_content.extend_from_slice(format!("tree {}\n", tree_id).as_bytes());
    commit_content.extend_from_slice(format!("parent {}\n", current_id).as_bytes());
    commit_content.extend_from_slice(format!("parent {}\n", source_id).as_bytes());
    commit_content.extend_from_slice(
        format!("author Unknown <unknown> {} +0000\n", timestamp).as_bytes(),
    );
    commit_content.extend_from_slice(
        format!("committer Unknown <unknown> {} +0000\n", timestamp).as_bytes(),
    );
    commit_content.extend_from_slice(b"\n");
    commit_content
        .extend_from_slice(format!("Merge branch '{}'\n", source_branch).as_bytes());

    let mut commit_obj = Vec::new();
    commit_obj.extend_from_slice(b"CRUST-OBJECT\n");
    commit_obj.extend_from_slice(b"type: commit\n");
    commit_obj.extend_from_slice(format!("size: {}\n\n", commit_content.len()).as_bytes());
    commit_obj.extend_from_slice(&commit_content);

    let mut hasher = Sha256::new();
    hasher.update(&commit_obj);
    let commit_id = format!("{:x}", hasher.finalize());
    save_object(repo_root, &commit_id, &commit_obj)?;

    refs::update_branch(repo_root, branch, &commit_id)?;

    let empty_index = index::Index::new();
    empty_index.save(repo_root)?;

    Ok(commit_id)
}

fn save_object(repo_root: &str, object_id: &str, data: &[u8]) -> Result<()> {
    let dir = format!("{}/.crust/objects/{}", repo_root, &object_id[0..2]);
    fs::create_dir_all(&dir)?;
    let path = format!("{}/{}", dir, &object_id[2..]);
    if !Path::new(&path).exists() {
        let compressed = zstd::encode_all(data, 3)?;
        fs::write(&path, compressed)?;
    }
    Ok(())
}
