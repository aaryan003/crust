// commit command - create a commit from staged changes

use crate::index::Index;
use crate::working_tree;
use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{self, BufRead, Write};
use std::path::Path;

/// Acquire an exclusive repo lock (.crust/LOCK).
/// Returns an error if another operation holds the lock.
fn acquire_lock(repo_root: &str) -> Result<String> {
    let lock_path = format!("{}/.crust/LOCK", repo_root);
    if Path::new(&lock_path).exists() {
        return Err(anyhow!(
            "Another crust operation is already in progress (lock file: {}). \
             If no other operation is running, remove it manually.",
            lock_path
        ));
    }
    fs::write(&lock_path, format!("{}", std::process::id()))
        .with_context(|| format!("Failed to create lock file: {}", lock_path))?;
    Ok(lock_path)
}

fn release_lock(lock_path: &str) {
    let _ = fs::remove_file(lock_path);
}

pub fn cmd_commit(message: Option<&str>) -> Result<()> {
    let repo_root = ".";

    if !std::path::Path::new(".crust").exists() {
        return Err(anyhow!("CLI_NO_REPOSITORY: Not in a CRUST repository"));
    }

    // Acquire exclusive lock — prevents concurrent commits (EDGE-10)
    let lock_path = acquire_lock(repo_root)?;
    let result = cmd_commit_inner(repo_root, message);
    release_lock(&lock_path);
    result
}

fn cmd_commit_inner(repo_root: &str, message: Option<&str>) -> Result<()> {
    let owned_message;
    let message_str: &str = if let Some(m) = message {
        if m.trim().is_empty() {
            return Err(anyhow!("Commit message cannot be empty"));
        }
        m
    } else {
        // Try to read from stdin (supports `crust commit` with piped input or interactive prompt)
        print!("Commit message: ");
        io::stdout().flush().ok();
        let stdin = io::stdin();
        let mut line = String::new();
        stdin.lock().read_line(&mut line)?;
        owned_message = line.trim().to_string();
        if owned_message.is_empty() {
            return Err(anyhow!("Aborting commit due to empty commit message."));
        }
        &owned_message
    };

    // Load index
    let index = Index::load(repo_root)?;

    if index.is_empty() {
        return Err(anyhow!("nothing to commit (working tree clean)"));
    }

    let message = message_str;
    let mut tree_content = Vec::new();
    for entry in index.entries() {
        // Add entry to tree (mode name\0sha256)
        tree_content.extend_from_slice(b"100644 ");
        tree_content.extend_from_slice(entry.path.as_bytes());
        tree_content.push(0);

        // Add the SHA256 bytes
        let sha_hex = &entry.blob_id;
        let sha_bytes =
            hex::decode(sha_hex).map_err(|_| anyhow!("Invalid blob ID: {}", sha_hex))?;

        if sha_bytes.len() != 32 {
            return Err(anyhow!("Invalid SHA256 in index"));
        }
        tree_content.extend_from_slice(&sha_bytes);
    }

    // Create tree object
    let mut tree_object = Vec::new();
    tree_object.extend_from_slice(b"CRUST-OBJECT\n");
    tree_object.extend_from_slice(b"type: tree\n");
    tree_object.extend_from_slice(format!("size: {}\n\n", tree_content.len()).as_bytes());
    tree_object.extend_from_slice(&tree_content);

    // Hash tree
    let mut hasher = Sha256::new();
    hasher.update(&tree_object);
    let tree_id = format!("{:x}", hasher.finalize());

    // Save tree object to disk
    save_object(repo_root, &tree_id, &tree_object)?;

    // Get parent commit (if any)
    let head_ref = working_tree::get_head_ref(repo_root)?;
    let parent = working_tree::read_ref(repo_root, &head_ref)?;

    // Create commit object
    let now = Utc::now();
    let timestamp = now.timestamp();
    let mut commit_content = Vec::new();

    commit_content.extend_from_slice(format!("tree {}\n", tree_id).as_bytes());

    if let Some(parent_id) = &parent {
        commit_content.extend_from_slice(format!("parent {}\n", parent_id).as_bytes());
    }

    // Author and committer (default to "Unknown")
    commit_content
        .extend_from_slice(format!("author Unknown <unknown> {} +0000\n", timestamp).as_bytes());
    commit_content
        .extend_from_slice(format!("committer Unknown <unknown> {} +0000\n", timestamp).as_bytes());

    commit_content.extend_from_slice(b"\n");
    commit_content.extend_from_slice(message.as_bytes());

    // Create commit object
    let mut commit_object = Vec::new();
    commit_object.extend_from_slice(b"CRUST-OBJECT\n");
    commit_object.extend_from_slice(b"type: commit\n");
    commit_object.extend_from_slice(format!("size: {}\n\n", commit_content.len()).as_bytes());
    commit_object.extend_from_slice(&commit_content);

    // Hash commit
    let mut hasher = Sha256::new();
    hasher.update(&commit_object);
    let commit_id = format!("{:x}", hasher.finalize());

    // Save commit object to disk
    save_object(repo_root, &commit_id, &commit_object)?;

    // Update HEAD ref
    working_tree::write_ref(repo_root, &head_ref, &commit_id)?;

    // Get branch name
    let branch = working_tree::get_current_branch(repo_root)?;
    let files_changed = index.entries().len();

    println!("[{} {}] {}", branch, &commit_id[..7], message);
    println!(" {} files changed", files_changed);

    // Keep index in sync with the committed tree (mirrors HEAD after commit)
    // This ensures `crust status` shows a clean state right after commit,
    // and correctly tracks deletions/modifications going forward.
    index.save(repo_root)?;

    Ok(())
}

/// Save an object to disk atomically using a temp file + rename.
/// Write order: temp file → rename to final path. This guarantees
/// no partial objects are visible even if the process is interrupted
/// mid-write (EDGE-08). The ref is only updated after all objects are
/// saved, so a crash at any earlier point leaves the repo in pre-commit
/// state with no visible change.
fn save_object(repo_root: &str, object_id: &str, object_data: &[u8]) -> Result<()> {
    // Create objects directory structure
    let objects_dir = format!("{}/.crust/objects", repo_root);
    fs::create_dir_all(&objects_dir)
        .with_context(|| format!("Cannot create objects directory '{}' (disk full or read-only?)", objects_dir))?;

    let subdir = format!("{}/{}", objects_dir, &object_id[0..2]);
    fs::create_dir_all(&subdir)
        .with_context(|| format!("Cannot create object subdir '{}'", subdir))?;

    let object_path = format!("{}/{}", subdir, &object_id[2..]);

    // Skip if already stored (content-addressed — same ID == same content)
    if Path::new(&object_path).exists() {
        return Ok(());
    }

    // Compress with zstd
    let mut compressed = Vec::new();
    let mut encoder = zstd::Encoder::new(&mut compressed, 0)?;
    std::io::Write::write_all(&mut encoder, object_data)?;
    encoder.finish()?;

    // Write to temp file first, then atomically rename (EDGE-08)
    let tmp_path = format!("{}.tmp", object_path);
    fs::write(&tmp_path, &compressed)
        .with_context(|| format!("Failed to write object (disk full?): {}", tmp_path))?;
    fs::rename(&tmp_path, &object_path)
        .with_context(|| format!("Failed to finalize object '{}' (disk full?)", object_path))?;

    Ok(())
}
