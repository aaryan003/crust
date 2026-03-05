// clone command - download repository from remote

use crate::commands::checkout::restore_working_tree_pub;
use crate::commands::fetch::unpack_and_store;
use crate::config::Config;
use crate::remote::RemoteSync;
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

pub fn cmd_clone(url: &str, directory: Option<&str>) -> Result<()> {
    // Parse URL
    let (owner, repo) = parse_repo_url(url)?;

    // Determine directory name
    let dir_name = directory.unwrap_or(&repo);

    // Check if directory already exists
    if Path::new(dir_name).exists() {
        return Err(anyhow!("Directory '{}' already exists", dir_name));
    }

    println!("Cloning into '{}'...", dir_name);

    // Create remote sync (to check auth and connectivity)
    let sync = RemoteSync::new(url, &owner, &repo)?;

    // Verify the repo exists on the server before creating local directory
    if let Err(e) = sync.check_repo_exists() {
        let msg = e.to_string();
        if msg.contains("401") || msg.contains("AUTH_REQUIRED") || msg.contains("Unauthorized") {
            return Err(anyhow!("AUTH_REQUIRED: Authentication required to clone '{}'", url));
        }
        return Err(anyhow!("REPO_NOT_FOUND: Repository '{}' not found on remote", url));
    }

    // Fetch remote refs to know what branches exist and their commit SHAs
    let (heads, default_branch) = fetch_remote_refs(url, &sync)?;

    if heads.is_empty() {
        // Repo exists but is empty
        fs::create_dir_all(dir_name)?;
        std::env::set_current_dir(dir_name)?;
        init_bare_repo(url)?;
        println!("Cloned empty repository.");
        std::env::set_current_dir("..")?;
        return Ok(());
    }

    // Fetch all objects (wants = all known branch tips)
    let wants: Vec<String> = heads.values().cloned().collect();
    println!("remote: Enumerating objects: {} refs", wants.len());

    let pack_data = sync.fetch(wants, vec![])?;

    // Create directory and initialize repo
    fs::create_dir_all(dir_name)?;
    std::env::set_current_dir(dir_name)?;

    init_bare_repo(url)?;

    // Unpack objects into .crust/objects/
    let stored = unpack_and_store(".", &pack_data)?;
    println!(
        "remote: Receiving objects: 100% ({}/{}), done.",
        stored, stored
    );

    // Write all remote refs to .crust/refs/heads/
    for (branch, commit_sha) in &heads {
        let ref_dir = format!(".crust/refs/heads");
        // Handle branch names with slashes (e.g. feat/auth)
        let branch_parts: Vec<&str> = branch.rsplitn(2, '/').collect();
        if branch_parts.len() > 1 {
            fs::create_dir_all(format!("{}/{}", ref_dir, branch_parts[1]))?;
        } else {
            fs::create_dir_all(&ref_dir)?;
        }
        fs::write(format!("{}/.crust/refs/heads/{}", ".", branch), format!("{}\n", commit_sha))?;
    }

    // Point HEAD to the default branch
    fs::write(
        ".crust/HEAD",
        format!("ref: refs/heads/{}\n", default_branch),
    )?;

    // Restore working tree from the HEAD commit
    if let Some(head_commit) = heads.get(&default_branch) {
        let _ = restore_working_tree_pub(".", head_commit);
    }

    println!("Checked out branch '{}'.", default_branch);

    std::env::set_current_dir("..")?;

    Ok(())
}

/// Initialize the .crust directory structure (without a remote fetch).
fn init_bare_repo(remote_url: &str) -> Result<()> {
    fs::create_dir_all(".crust/objects")?;
    fs::create_dir_all(".crust/refs/heads")?;
    fs::create_dir_all(".crust/refs/tags")?;
    fs::write(".crust/HEAD", "ref: refs/heads/main\n")?;
    fs::write(".crust/index", "{}")?;

    let mut config = Config::new();
    config.add_remote("origin".to_string(), remote_url.to_string())?;
    config.save()?;

    Ok(())
}

/// Call GET /api/v1/repos/:owner/:repo/refs to get all branch → commit mappings.
/// Returns (heads map, default_branch).
fn fetch_remote_refs(
    server_url: &str,
    sync: &RemoteSync,
) -> Result<(HashMap<String, String>, String)> {
    let refs_data = sync.get_refs()?;

    // Pick default branch (prefer "main", then first available)
    let default_branch = if refs_data.contains_key("main") {
        "main".to_string()
    } else {
        refs_data.keys().next().cloned().unwrap_or_else(|| "main".to_string())
    };

    let _ = server_url;
    Ok((refs_data, default_branch))
}

fn parse_repo_url(url: &str) -> Result<(String, String)> {
    // URL format: https://server.com/owner/repo or https://server.com/owner/repo.crust
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
