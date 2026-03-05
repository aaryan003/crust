// show command - display commit details and diff

use anyhow::{anyhow, Result};
use gitcore::Commit;
use std::fs;
use std::path::Path;

/// Show commit details and diff from parent
pub fn cmd_show(ref_spec: &str) -> Result<()> {
    let repo_root = ".";

    // Check if we're in a repo
    if !Path::new(".crust").exists() {
        return Err(anyhow!("CLI_NO_REPOSITORY: Not in a CRUST repository"));
    }

    // Resolve ref (branch name or commit ID)
    let commit_id = resolve_ref(repo_root, ref_spec)?;

    // Load commit object from disk
    let commit = load_commit_object(repo_root, &commit_id)?;

    println!("commit {}", commit_id);
    println!("Author: {}", commit.author);

    // Format timestamp
    let date_str = format_date(commit.timestamp, &commit.tz_offset);
    println!("Date:   {}", date_str);
    println!();
    for line in commit.message.lines() {
        println!("    {}", line);
    }
    println!();

    // Show tree id
    println!("tree {}", commit.tree.as_str());
    if !commit.parents.is_empty() {
        println!("parent {}", commit.parents[0].as_str());
    }

    Ok(())
}

/// Resolve a ref to a commit ID (branch name or commit ID)
fn resolve_ref(repo_root: &str, ref_spec: &str) -> Result<String> {
    // Handle HEAD specially: follow the symref
    if ref_spec == "HEAD" {
        let head_path = format!("{}/.crust/HEAD", repo_root);
        let head_content = fs::read_to_string(&head_path)
            .map_err(|_| anyhow!("Failed to read HEAD"))?;
        let head_content = head_content.trim();
        if let Some(branch) = head_content.strip_prefix("ref: refs/heads/") {
            let branch_path = format!("{}/.crust/refs/heads/{}", repo_root, branch);
            if Path::new(&branch_path).exists() {
                let commit_id = fs::read_to_string(&branch_path)?;
                return Ok(commit_id.trim().to_string());
            }
            return Err(anyhow!("HEAD points to branch '{}' which has no commits", branch));
        }
        // Detached HEAD — content is the commit ID itself
        return Ok(head_content.to_string());
    }

    // Check if it's a branch name
    let branch_path = format!("{}/.crust/refs/heads/{}", repo_root, ref_spec);
    if Path::new(&branch_path).exists() {
        let commit_id = fs::read_to_string(&branch_path)?;
        return Ok(commit_id.trim().to_string());
    }

    // Otherwise treat as direct commit ID (validate it exists)
    let object_path = format!(
        "{}/.crust/objects/{}/{}",
        repo_root,
        &ref_spec[0..2],
        &ref_spec[2..]
    );
    if Path::new(&object_path).exists() {
        return Ok(ref_spec.to_string());
    }

    Err(anyhow!("Ref not found: {}", ref_spec))
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
