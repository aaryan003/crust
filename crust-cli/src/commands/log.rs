// log command - show commit history

use crate::working_tree;
use anyhow::{anyhow, Result};
use gitcore::Commit;
use std::fs;
use std::path::Path;

/// Show full commit history from HEAD, newest first
pub fn cmd_log(branch: Option<&str>, max_count: Option<usize>) -> Result<()> {
    cmd_log_internal(false, branch, max_count)
}

/// Show commits in compact oneline format
pub fn cmd_log_oneline(branch: Option<&str>, max_count: Option<usize>) -> Result<()> {
    cmd_log_internal(true, branch, max_count)
}

fn cmd_log_internal(oneline: bool, branch: Option<&str>, max_count: Option<usize>) -> Result<()> {
    let repo_root = ".";

    // Check if we're in a repo
    if !Path::new(".crust").exists() {
        return Err(anyhow!("CLI_NO_REPOSITORY: Not in a CRUST repository"));
    }

    let current_branch = working_tree::get_current_branch(repo_root)?;

    // Resolve to a starting commit
    let start_commit = if let Some(b) = branch {
        if let Ok(Some(sha)) = working_tree::read_ref(repo_root, &format!("refs/heads/{}", b)) {
            sha
        } else if let Ok(Some(sha)) = working_tree::read_ref(repo_root, &format!("refs/remotes/{}", b)) {
            sha
        } else if let Ok(Some(sha)) = working_tree::read_ref(repo_root, b) {
            sha
        } else {
            return Err(anyhow!("Branch or ref '{}' not found", b));
        }
    } else {
        let head_ref = working_tree::get_head_ref(repo_root)?;
        match working_tree::read_ref(repo_root, &head_ref)? {
            Some(id) => id,
            None => {
                println!("No commits in branch '{}'", current_branch);
                return Ok(());
            }
        }
    };

    traverse_commits(repo_root, &start_commit, oneline, max_count)?;
    Ok(())
}

/// Traverse commit history from a starting commit ID, following parent chain
fn traverse_commits(repo_root: &str, start_id: &str, oneline: bool, max_count: Option<usize>) -> Result<()> {
    let mut current_id = start_id.to_string();
    let mut visited = std::collections::HashSet::new();
    let mut count = 0usize;

    loop {
        // Enforce max count
        if let Some(limit) = max_count {
            if count >= limit {
                break;
            }
        }

        // Prevent infinite loops
        if visited.contains(&current_id) {
            break;
        }
        visited.insert(current_id.clone());

        // Try to load and parse the commit object
        match load_commit_object(repo_root, &current_id) {
            Ok(commit) => {
                if oneline {
                    // Compact format: {short_id} {first_line_of_message}
                    let short_id = &current_id[0..7];
                    let first_line = commit.message.lines().next().unwrap_or("").trim();
                    println!("{} {}", short_id, first_line);
                } else {
                    // Full format
                    println!("commit {}", current_id);
                    println!("Author: {}", commit.author);

                    // Format timestamp with timezone
                    let date_str = format_date(commit.timestamp, &commit.tz_offset);
                    println!("Date:   {}", date_str);
                    println!();

                    // Indent message
                    for line in commit.message.lines() {
                        println!("    {}", line);
                    }
                    println!();
                }
                count += 1;

                // Move to parent (if exists)
                if !commit.parents.is_empty() {
                    current_id = commit.parents[0].as_str().to_string();
                } else {
                    // Root commit, no more parents
                    break;
                }
            }
            Err(_) => {
                // Can't load commit object, stop traversal
                break;
            }
        }
    }

    Ok(())
}

/// Load a commit object from disk
fn load_commit_object(repo_root: &str, commit_id: &str) -> Result<Commit> {
    let object_path = format!(
        "{}/.crust/objects/{}/{}",
        repo_root,
        &commit_id[0..2],
        &commit_id[2..]
    );

    if !Path::new(&object_path).exists() {
        return Err(anyhow!("Commit object not found: {}", commit_id));
    }

    // Read compressed object
    let compressed = fs::read(&object_path)?;

    // Decompress
    let decompressed = zstd::decode_all(&compressed[..])?;

    // Parse object header and content
    let content = parse_object_content(&decompressed)?;

    // Deserialize commit
    let commit = Commit::deserialize(&content)?;

    Ok(commit)
}

/// Parse CRUST object format: extract content after header
fn parse_object_content(data: &[u8]) -> Result<Vec<u8>> {
    // Format:
    // CRUST-OBJECT\n
    // type: {type}\n
    // size: {size}\n
    // \n
    // {content}

    let text = std::str::from_utf8(data)?;
    let mut lines = text.lines();

    // Skip "CRUST-OBJECT"
    if lines.next() != Some("CRUST-OBJECT") {
        return Err(anyhow!(
            "Invalid object format: missing CRUST-OBJECT header"
        ));
    }

    // Skip type line
    lines.next();

    // Skip size line
    lines.next();

    // Skip blank line
    lines.next();

    // Collect remaining as content
    let content_str = lines.collect::<Vec<_>>().join("\n");
    Ok(content_str.into_bytes())
}

/// Format Unix timestamp as human-readable date string
fn format_date(timestamp: i64, tz_offset: &str) -> String {
    // Simple approximation for display (would need proper date library for production)
    // For now, just show timestamp with timezone
    format!(
        "timestamp: {} UTC{}",
        timestamp,
        if tz_offset.is_empty() {
            "+0000".to_string()
        } else {
            tz_offset.to_string()
        }
    )
}
