// rev-parse command - resolve a reference and print its commit ID

use anyhow::{anyhow, Result};
use std::fs;
use std::path::Path;

/// Resolve a reference (HEAD, branch name, or commit SHA) and print its commit ID
/// 
/// Arguments:
///   - ref_name: "HEAD", branch name (e.g. "main"), or raw commit SHA256 (64 hex chars)
/// 
/// Output: Prints the resolved 64-character SHA256 commit ID
/// 
/// Errors:
///   - CLI_NOT_IN_REPO: Not in a CRUST repository
///   - CLI_REF_NOT_FOUND: Reference doesn't exist or can't be resolved
///   - VALIDATE_INVALID_FORMAT: Invalid commit ID format
pub fn cmd_rev_parse(ref_name: &str) -> Result<()> {
    // Check if we're in a repo
    if !Path::new(".crust").exists() {
        return Err(anyhow!("CLI_NOT_IN_REPO: Not in a CRUST repository"));
    }

    let resolved_id = resolve_reference(ref_name)?;
    println!("{}", resolved_id);
    Ok(())
}

/// Resolve a reference to a commit SHA256
/// 
/// Handles three cases:
/// 1. "HEAD" → reads .crust/HEAD, follows symref to branch, reads branch file
/// 2. Branch name (e.g. "main") → reads .crust/refs/heads/main
/// 3. Raw SHA256 (64 hex chars) → validates and passes through
fn resolve_reference(ref_name: &str) -> Result<String> {
    // Case 3: Raw SHA256 (64 hex chars) - pass through after validation
    if ref_name.len() == 64 && ref_name.chars().all(|c| c.is_ascii_hexdigit()) {
        return Ok(ref_name.to_string());
    }

    // Case 1: HEAD reference
    if ref_name == "HEAD" {
        return resolve_head();
    }

    // Case 2: Branch name
    let branch_path = format!(".crust/refs/heads/{}", ref_name);
    if Path::new(&branch_path).exists() {
        let commit_id = fs::read_to_string(&branch_path)
            .map_err(|e| anyhow!("CLI_REF_NOT_FOUND: Failed to read branch '{}': {}", ref_name, e))?;
        let commit_id = commit_id.trim().to_string();
        
        if commit_id.is_empty() {
            return Err(anyhow!("CLI_REF_NOT_FOUND: Branch '{}' points to empty commit", ref_name));
        }

        return Ok(commit_id);
    }

    // Reference not found
    Err(anyhow!("CLI_REF_NOT_FOUND: Reference '{}' not found", ref_name))
}

/// Resolve the HEAD reference to a commit SHA256
/// 
/// HEAD file format: "ref: refs/heads/main"
/// Follows the symref and reads the actual branch file
fn resolve_head() -> Result<String> {
    let head_path = ".crust/HEAD";
    
    if !Path::new(head_path).exists() {
        return Err(anyhow!("CLI_REF_NOT_FOUND: HEAD not found (no commits yet?)"));
    }

    let head_content = fs::read_to_string(head_path)
        .map_err(|e| anyhow!("CLI_REF_NOT_FOUND: Failed to read HEAD: {}", e))?;

    let head_content = head_content.trim();

    // Format: "ref: refs/heads/main"
    if let Some(branch_path) = head_content.strip_prefix("ref: ") {
        if let Some(branch_name) = branch_path.strip_prefix("refs/heads/") {
            let branch_file = format!(".crust/refs/heads/{}", branch_name);
            
            if !Path::new(&branch_file).exists() {
                return Err(anyhow!(
                    "CLI_REF_NOT_FOUND: HEAD points to branch '{}' which doesn't exist",
                    branch_name
                ));
            }

            let commit_id = fs::read_to_string(&branch_file)
                .map_err(|e| anyhow!("CLI_REF_NOT_FOUND: Failed to read branch file: {}", e))?;
            
            let commit_id = commit_id.trim().to_string();
            if commit_id.is_empty() {
                return Err(anyhow!("CLI_REF_NOT_FOUND: Branch '{}' has no commits", branch_name));
            }

            return Ok(commit_id);
        }
    }

    // HEAD could also directly contain a commit ID (detached state)
    // In that case, use it directly
    if head_content.len() == 64 && head_content.chars().all(|c| c.is_ascii_hexdigit()) {
        return Ok(head_content.to_string());
    }

    Err(anyhow!(
        "CLI_REF_NOT_FOUND: Invalid HEAD format (expected 'ref: refs/heads/<branch>' or SHA256)"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_resolve_raw_sha() -> Result<()> {
        let sha = "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789";
        let resolved = resolve_reference(sha)?;
        assert_eq!(resolved, sha);
        Ok(())
    }

    #[test]
    fn test_resolve_head() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let orig_cwd = std::env::current_dir()?;
        std::env::set_current_dir(temp_dir.path())?;

        let sha = "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";
        fs::create_dir_all(".crust/refs/heads")?;
        fs::write(".crust/refs/heads/main", format!("{}\n", sha))?;
        fs::write(".crust/HEAD", "ref: refs/heads/main\n")?;

        let resolved = resolve_reference("HEAD")?;
        assert_eq!(resolved, sha);

        std::env::set_current_dir(orig_cwd)?;
        Ok(())
    }

    #[test]
    fn test_resolve_branch() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let orig_cwd = std::env::current_dir()?;
        std::env::set_current_dir(temp_dir.path())?;

        let sha = "fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210";
        fs::create_dir_all(".crust/refs/heads")?;
        fs::write(".crust/refs/heads/develop", format!("{}\n", sha))?;

        let resolved = resolve_reference("develop")?;
        assert_eq!(resolved, sha);

        std::env::set_current_dir(orig_cwd)?;
        Ok(())
    }

    #[test]
    fn test_resolve_nonexistent() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let orig_cwd = std::env::current_dir()?;
        std::env::set_current_dir(temp_dir.path())?;

        fs::create_dir(".crust")?;

        let result = resolve_reference("nonexistent");
        assert!(result.is_err());

        std::env::set_current_dir(orig_cwd)?;
        Ok(())
    }

    #[test]
    fn test_invalid_format() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let orig_cwd = std::env::current_dir()?;
        std::env::set_current_dir(temp_dir.path())?;

        fs::create_dir(".crust")?;

        // SHA too short
        let result = resolve_reference("abc123");
        assert!(result.is_err());

        // Invalid hex
        let result = resolve_reference("zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz");
        assert!(result.is_err());

        std::env::set_current_dir(orig_cwd)?;
        Ok(())
    }
}
