// Refs module - handles branch and ref management

use anyhow::{anyhow, Result};
use std::fs;
use std::path::Path;

/// List all branches in the repository
pub fn list_branches(repo_root: &str) -> Result<Vec<String>> {
    let heads_dir = format!("{}/.crust/refs/heads", repo_root);

    if !Path::new(&heads_dir).exists() {
        return Ok(Vec::new());
    }

    let mut branches = Vec::new();
    collect_branches(&heads_dir, &heads_dir, &mut branches)?;
    branches.sort();
    Ok(branches)
}

/// Recursively collect branch names relative to the heads directory
fn collect_branches(base: &str, dir: &str, branches: &mut Vec<String>) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_branches(base, path.to_str().unwrap_or(""), branches)?;
        } else if path.is_file() {
            // Make name relative to heads_dir
            let rel = path.strip_prefix(base)
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();
            if !rel.is_empty() {
                branches.push(rel);
            }
        }
    }
    Ok(())
}

/// Get current branch name
pub fn get_current_branch(repo_root: &str) -> Result<String> {
    let head_path = format!("{}/.crust/HEAD", repo_root);
    let content = fs::read_to_string(&head_path)?;

    // HEAD format: "ref: refs/heads/main"
    if let Some(branch_path) = content.trim().strip_prefix("ref: ") {
        if let Some(branch_name) = branch_path.strip_prefix("refs/heads/") {
            return Ok(branch_name.to_string());
        }
    }

    Err(anyhow!("Invalid HEAD format"))
}

/// Get the commit ID pointed to by a branch
#[allow(dead_code)]
pub fn get_branch_head(repo_root: &str, branch: &str) -> Result<Option<String>> {
    let ref_path = format!("{}/.crust/refs/heads/{}", repo_root, branch);

    if !Path::new(&ref_path).exists() {
        return Ok(None);
    }

    let commit_id = fs::read_to_string(&ref_path)?;
    Ok(Some(commit_id.trim().to_string()))
}

/// Create a new branch at the given commit
pub fn create_branch(repo_root: &str, branch: &str, commit_id: &str) -> Result<()> {
    let ref_path = format!("{}/.crust/refs/heads/{}", repo_root, branch);

    if Path::new(&ref_path).exists() {
        return Err(anyhow!("Branch '{}' already exists", branch));
    }

    // Create parent directories (supports names like feat/auth)
    if let Some(parent) = std::path::Path::new(&ref_path).parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(&ref_path, format!("{}\n", commit_id))?;
    Ok(())
}

/// Delete a branch
pub fn delete_branch(repo_root: &str, branch: &str) -> Result<()> {
    let ref_path = format!("{}/.crust/refs/heads/{}", repo_root, branch);

    if !Path::new(&ref_path).exists() {
        return Err(anyhow!("Branch '{}' does not exist", branch));
    }

    fs::remove_file(&ref_path)?;
    Ok(())
}

/// Switch to a branch by updating HEAD
pub fn switch_branch(repo_root: &str, branch: &str) -> Result<()> {
    // Check branch exists
    if !Path::new(&format!("{}/.crust/refs/heads/{}", repo_root, branch)).exists() {
        return Err(anyhow!("Branch '{}' does not exist", branch));
    }

    let head_path = format!("{}/.crust/HEAD", repo_root);
    fs::write(&head_path, format!("ref: refs/heads/{}\n", branch))?;
    Ok(())
}

/// Update a branch's commit ID
#[allow(dead_code)]
pub fn update_branch(repo_root: &str, branch: &str, commit_id: &str) -> Result<()> {
    let ref_path = format!("{}/.crust/refs/heads/{}", repo_root, branch);
    fs::write(&ref_path, format!("{}\n", commit_id))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_branch_operations() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let repo_root = temp_dir.path().to_str().unwrap();

        // Create .crust structure
        fs::create_dir_all(format!("{}/.crust/refs/heads", repo_root))?;
        fs::write(
            format!("{}/.crust/HEAD", repo_root),
            "ref: refs/heads/main\n",
        )?;

        // Create initial commit
        let commit_id = "abc123def456abc123def456abc123def456abc123def456abc123def456abc1";
        create_branch(repo_root, "main", commit_id)?;

        // List branches
        let branches = list_branches(repo_root)?;
        assert_eq!(branches, vec!["main"]);

        // Get current branch
        let current = get_current_branch(repo_root)?;
        assert_eq!(current, "main");

        // Create new branch
        create_branch(repo_root, "dev", commit_id)?;
        let branches = list_branches(repo_root)?;
        assert_eq!(branches.len(), 2);

        // Switch branch
        switch_branch(repo_root, "dev")?;
        let current = get_current_branch(repo_root)?;
        assert_eq!(current, "dev");

        // Delete branch
        delete_branch(repo_root, "dev")?;
        let branches = list_branches(repo_root)?;
        assert_eq!(branches.len(), 1);

        Ok(())
    }
}
