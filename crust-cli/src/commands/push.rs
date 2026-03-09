// push command - upload commits to remote

use crate::config::Config;
use crate::pack::PackWriter;
use crate::remote::{RefUpdate, RemoteSync};
use crate::working_tree;
use anyhow::{anyhow, Result};
use gitcore::{Commit, Tree};
use std::collections::{HashSet, VecDeque};
use std::fs;
use std::path::Path;

pub fn cmd_push(remote: Option<&str>, branch: Option<&str>, force: bool, _set_upstream: bool) -> Result<()> {
    // Check if in a repo
    let repo_root = ".";
    if !Path::new(".crust").exists() {
        return Err(anyhow!("CLI_NO_REPOSITORY: Not in a CRUST repository"));
    }

    // Load config to get remote URL
    let config = Config::load()?;
    let remote_name = remote.unwrap_or("origin");
    let remote_url = config
        .get_remote(remote_name)
        .ok_or_else(|| anyhow!("No remote '{}' configured", remote_name))?;

    // Parse URL to get owner/repo
    let (owner, repo) = parse_repo_url(&remote_url)?;

    // Determine branch
    let branch_name = match branch {
        Some(b) => b.to_string(),
        None => working_tree::get_current_branch(repo_root)?,
    };

    // Get local HEAD commit for this branch
    let ref_path = format!("refs/heads/{}", branch_name);
    let local_commit = match working_tree::read_ref(repo_root, &ref_path)? {
        Some(sha) => sha,
        None => {
            println!("Nothing to push: branch '{}' has no commits", branch_name);
            return Ok(());
        }
    };

    println!(
        "Pushing branch '{}' to {}...",
        branch_name, remote_name
    );

    // Create remote sync
    let sync = RemoteSync::new(&remote_url, &owner, &repo)?;

    // Auth check: require authentication before any network operations
    if sync.token().is_none() {
        return Err(anyhow!(
            "CLI_NOT_AUTHENTICATED: Authentication required. Run `crust login {}` first.",
            &remote_url
        ));
    }

    // Fast "up-to-date" check: compare remote branch tip to local
    let remote_heads = sync.get_refs().unwrap_or_default();
    let remote_tip = remote_heads.get(&branch_name).cloned();

    if !force {
        if remote_tip.as_deref() == Some(local_commit.as_str()) {
            println!("Everything up-to-date");
            // Still update local remote-tracking ref even if up-to-date
            let tracking_ref_path = format!(".crust/refs/remotes/{}/{}", remote_name, branch_name);
            if let Some(parent) = std::path::Path::new(&tracking_ref_path).parent() {
                let _ = fs::create_dir_all(parent);
            }
            let _ = fs::write(&tracking_ref_path, &local_commit);
            return Ok(());
        }

        // Non-fast-forward check: if remote has commits not in our history, reject
        if let Some(ref remote_sha) = remote_tip {
            if !is_ancestor(repo_root, remote_sha, &local_commit)? {
                return Err(anyhow!(
                    "Push rejected: remote has commits not in your local history.\n\
                     Hint: Pull first with `crust pull origin {}` to integrate remote changes.",
                    branch_name
                ));
            }
        }
    }

    // Collect all objects reachable from our commit.
    let empty_known: HashSet<String> = HashSet::new();
    let objects_to_send = collect_reachable_objects(repo_root, &local_commit, &empty_known)?;

    if objects_to_send.is_empty() {
        println!("Everything up-to-date");
        return Ok(());
    }

    // Build pack
    let mut writer = PackWriter::new();
    for (object_id, object_type, data) in &objects_to_send {
        writer.add_object(
            object_id.clone(),
            object_type.clone(),
            data.clone(),
        )?;
    }

    let pack_data = writer.serialize()?;

    println!(
        "Sending {} object(s) ({} bytes packed)...",
        objects_to_send.len(),
        pack_data.len()
    );

    // Upload pack to server
    let upload_result = sync.upload(pack_data)?;

    if !upload_result.conflicts.is_empty() {
        eprintln!("Warning: {} object(s) had conflicts:", upload_result.conflicts.len());
        for c in &upload_result.conflicts {
            eprintln!("  {}", c);
        }
    }

    // Update remote ref
    let null_sha = "0000000000000000000000000000000000000000000000000000000000000000";
    let old_sha = remote_tip.as_deref().unwrap_or(null_sha).to_string();
    let updates = vec![RefUpdate {
        ref_name: format!("refs/heads/{}", branch_name),
        old_sha,
        new_sha: local_commit.clone(),
        force,
    }];

    let update_results = sync.update_refs(updates)?;
    for result in &update_results {
        if !result.ok {
            let reason = result.error.as_deref().unwrap_or("unknown error");
            return Err(anyhow!("Push rejected: {}", reason));
        }
    }

    println!(
        "remote: {} -> {} ({} objects)",
        branch_name,
        &local_commit[..12],
        upload_result.objects_stored
    );

    // Update local remote-tracking ref: .crust/refs/remotes/{remote}/{branch}
    let tracking_ref_path = format!(".crust/refs/remotes/{}/{}", remote_name, branch_name);
    if let Some(parent) = std::path::Path::new(&tracking_ref_path).parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(&tracking_ref_path, &local_commit);

    Ok(())
}

/// Collect all objects reachable from `start_commit` that are NOT in `server_known`.
/// Returns vec of (object_id_hex, object_type, raw_uncompressed_bytes).
fn collect_reachable_objects(
    repo_root: &str,
    start_commit: &str,
    server_known: &HashSet<String>,
) -> Result<Vec<(String, String, Vec<u8>)>> {
    let mut result: Vec<(String, String, Vec<u8>)> = Vec::new();
    let mut visited: HashSet<String> = HashSet::new();

    // BFS queue: (object_id, object_type)
    let mut queue: VecDeque<(String, String)> = VecDeque::new();
    queue.push_back((start_commit.to_string(), "commit".to_string()));

    while let Some((obj_id, obj_type)) = queue.pop_front() {
        if visited.contains(&obj_id) {
            continue;
        }
        visited.insert(obj_id.clone());

        // Skip if server already has it
        if server_known.contains(&obj_id) {
            continue;
        }

        // Load raw object bytes from disk
        let raw = match load_raw_object(repo_root, &obj_id) {
            Ok(data) => data,
            Err(_) => continue, // skip missing objects
        };

        // For commits: parse and enqueue tree + parents
        // For trees: parse and enqueue blob/subtree entries
        match obj_type.as_str() {
            "commit" => {
                if let Ok(content) = extract_object_content(&raw) {
                    if let Ok(commit) = Commit::deserialize(&content) {
                        queue.push_back((commit.tree.as_str().to_string(), "tree".to_string()));
                        for parent in &commit.parents {
                            queue.push_back((parent.as_str().to_string(), "commit".to_string()));
                        }
                    }
                }
            }
            "tree" => {
                if let Ok(content) = extract_object_content(&raw) {
                    if let Ok(tree) = Tree::deserialize(&content) {
                        for entry in tree.entries() {
                            let entry_type = if entry.mode == "040000" { "tree" } else { "blob" };
                            queue.push_back((entry.id.as_str().to_string(), entry_type.to_string()));
                        }
                    }
                }
            }
            _ => {} // blob, tag — no children to enqueue
        }

        result.push((obj_id, obj_type, raw));
    }

    Ok(result)
}

/// Load the raw (uncompressed) object bytes from local object store.
fn load_raw_object(repo_root: &str, object_id: &str) -> Result<Vec<u8>> {
    if object_id.len() < 3 {
        return Err(anyhow!("Invalid object ID: {}", object_id));
    }
    let path = format!(
        "{}/.crust/objects/{}/{}",
        repo_root,
        &object_id[0..2],
        &object_id[2..]
    );
    let compressed = fs::read(&path)
        .map_err(|_| anyhow!("Object not found: {}", object_id))?;
    let decompressed = zstd::decode_all(&compressed[..])
        .map_err(|e| anyhow!("Decompression failed for {}: {}", object_id, e))?;
    Ok(decompressed)
}

/// Extract the content bytes that follow the CRUST-OBJECT header.
/// Header ends at the first "\n\n" (blank line after "size: N").
/// Safe for binary content (tree objects).
fn extract_object_content(data: &[u8]) -> Result<Vec<u8>> {
    if let Some(pos) = data.windows(2).position(|w| w == b"\n\n") {
        Ok(data[pos + 2..].to_vec())
    } else {
        Err(anyhow!("Invalid object: no header separator found"))
    }
}

fn parse_repo_url(url: &str) -> Result<(String, String)> {
    let url_without_scheme = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .ok_or_else(|| anyhow!("Invalid URL format"))?;

    let parts: Vec<&str> = url_without_scheme.split('/').collect();
    if parts.len() < 3 {
        return Err(anyhow!("Invalid repository URL"));
    }

    let owner = parts[1].to_string();
    let repo = parts[2].trim_end_matches(".crust").to_string();

    Ok((owner, repo))
}

/// Check if `ancestor_sha` is an ancestor of `descendant_sha` by BFS walk.
/// Returns true if ancestor is reachable from descendant.
fn is_ancestor(repo_root: &str, ancestor_sha: &str, descendant_sha: &str) -> Result<bool> {
    if ancestor_sha == descendant_sha {
        return Ok(true);
    }

    let mut queue: VecDeque<String> = VecDeque::new();
    let mut visited: HashSet<String> = HashSet::new();
    queue.push_back(descendant_sha.to_string());

    while let Some(current) = queue.pop_front() {
        if visited.contains(&current) {
            continue;
        }
        visited.insert(current.clone());

        if current == ancestor_sha {
            return Ok(true);
        }

        // Read commit object to get parent(s)
        if current.len() < 4 {
            continue;
        }
        let obj_path = format!("{}/.crust/objects/{}/{}", repo_root, &current[..2], &current[2..]);
        let compressed = match fs::read(&obj_path) {
            Ok(d) => d,
            Err(_) => continue,
        };
        let raw = match zstd::decode_all(compressed.as_slice()) {
            Ok(d) => d,
            Err(_) => continue,
        };
        let text = String::from_utf8_lossy(&raw);

        // The object format is: "CRUST-OBJECT\ntype: commit\nsize: N\n\n<content>"
        // Skip the header section (everything before the double newline)
        let content = if let Some(pos) = text.find("\n\n") {
            &text[pos + 2..]
        } else {
            &text
        };

        // Parse parent lines from the commit content
        for line in content.lines() {
            if let Some(parent_sha) = line.strip_prefix("parent ") {
                let parent_sha = parent_sha.trim();
                if parent_sha.len() == 64 {
                    queue.push_back(parent_sha.to_string());
                }
            }
            // Stop at the blank line separating headers from commit message
            if line.is_empty() {
                break;
            }
        }
    }

    Ok(false)
}
