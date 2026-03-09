// Working tree module - handles filesystem operations and blob creation

use anyhow::{anyhow, Result};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

/// Represents a file in the working tree
#[derive(Debug, Clone)]
pub struct WorkingTreeFile {
    /// Path relative to repo root
    pub path: String,
    /// File content
    pub content: Vec<u8>,
    /// File size
    pub size: u64,
    /// Last modified time (unix timestamp)
    pub mtime: u64,
}

impl WorkingTreeFile {
    /// Compute SHA256 blob ID for this file
    pub fn blob_id(&self) -> String {
        // Create blob object with CRUST format
        let mut blob_data = Vec::new();
        blob_data.extend_from_slice(b"CRUST-OBJECT\n");
        blob_data.extend_from_slice(b"type: blob\n");
        blob_data.extend_from_slice(format!("size: {}\n\n", self.content.len()).as_bytes());
        blob_data.extend_from_slice(&self.content);

        // Hash it
        let mut hasher = Sha256::new();
        hasher.update(&blob_data);
        format!("{:x}", hasher.finalize())
    }
}

/// Walk the working tree and find modified/new files
pub fn scan_working_tree(repo_root: &str, path_spec: Option<&str>) -> Result<Vec<WorkingTreeFile>> {
    let mut files = Vec::new();

    if let Some(".") = path_spec {
        // Stage all files
        scan_directory(repo_root, repo_root, &mut files)?;
    } else if let Some(path) = path_spec {
        // Stage specific file or directory
        let full_path = format!("{}/{}", repo_root, path);

        if Path::new(&full_path).is_file() {
            let file = load_file(&full_path, path)?;
            files.push(file);
        } else if Path::new(&full_path).is_dir() {
            scan_directory(repo_root, &full_path, &mut files)?;
        } else {
            return Err(anyhow!("{}: No such file or directory", path));
        }
    }

    Ok(files)
}

/// Recursively scan directory for files
fn scan_directory(repo_root: &str, dir: &str, files: &mut Vec<WorkingTreeFile>) -> Result<()> {
    let entries = fs::read_dir(dir)?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        // Skip .crust directory
        if path.file_name().is_some_and(|n| n == ".crust") {
            continue;
        }

        // Skip hidden files (except .crust)
        if path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.starts_with('.'))
        {
            continue;
        }

        if path.is_file() {
            let rel_path = path
                .strip_prefix(repo_root)?
                .to_string_lossy()
                .trim_start_matches('/')
                .to_string();

            let file = load_file(path.to_str().unwrap(), &rel_path)?;
            files.push(file);
        } else if path.is_dir() {
            scan_directory(repo_root, path.to_str().unwrap(), files)?;
        }
    }

    Ok(())
}

/// Load a single file from disk
fn load_file(file_path: &str, rel_path: &str) -> Result<WorkingTreeFile> {
    let metadata = fs::metadata(file_path)?;
    let content = fs::read(file_path)?;
    let size = metadata.len();
    let mtime = metadata
        .modified()?
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();

    Ok(WorkingTreeFile {
        path: rel_path.to_string(),
        content,
        size,
        mtime,
    })
}

/// Get the current HEAD reference
pub fn get_head_ref(repo_root: &str) -> Result<String> {
    let head_file = format!("{}/.crust/HEAD", repo_root);
    let contents = fs::read_to_string(&head_file)?;

    if let Some(ref_path) = contents.strip_prefix("ref: ") {
        Ok(ref_path.trim().to_string())
    } else {
        // Detached HEAD (direct commit ID)
        Ok(contents.trim().to_string())
    }
}

/// Get the current branch name from HEAD
pub fn get_current_branch(repo_root: &str) -> Result<String> {
    let head_ref = get_head_ref(repo_root)?;

    if head_ref.starts_with("refs/heads/") {
        Ok(head_ref.strip_prefix("refs/heads/").unwrap().to_string())
    } else {
        Ok("(detached HEAD)".to_string())
    }
}

/// Read a ref file
pub fn read_ref(repo_root: &str, ref_path: &str) -> Result<Option<String>> {
    let full_path = format!("{}/.crust/{}", repo_root, ref_path);

    match fs::read_to_string(&full_path) {
        Ok(content) => Ok(Some(content.trim().to_string())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// Write a ref file
pub fn write_ref(repo_root: &str, ref_path: &str, object_id: &str) -> Result<()> {
    let full_path = format!("{}/.crust/{}", repo_root, ref_path);

    // Ensure parent directories exist
    if let Some(parent) = Path::new(&full_path).parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(&full_path, format!("{}\n", object_id))?;
    Ok(())
}
