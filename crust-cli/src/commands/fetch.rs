// fetch command - download objects from remote

use crate::config::Config;
use crate::pack::PackReader;
use crate::remote::RemoteSync;
use anyhow::{anyhow, Result};
use std::fs;
use std::path::Path;

pub fn cmd_fetch(remote: Option<&str>, branch: Option<&str>) -> Result<()> {
    // Check if in a repo
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

    println!("Fetching from {}...", remote_name);

    // Create remote sync
    let sync = RemoteSync::new(&remote_url, &owner, &repo)?;

    // Get remote refs — what branch tips does the server have?
    let remote_heads_all = sync.get_refs()
        .map_err(|e| anyhow!("Failed to get remote refs: {}", e))?;

    // If specific branch requested, filter
    let remote_heads: std::collections::HashMap<String, String> = if let Some(b) = branch {
        remote_heads_all.into_iter().filter(|(k, _)| k == b).collect()
    } else {
        remote_heads_all
    };

    if remote_heads.is_empty() {
        println!("Already up to date.");
        return Ok(());
    }

    // Compare remote branch tips against our local tracking refs.
    // Collect wants = remote SHAs we don't already have locally.
    let haves = collect_local_object_ids(".crust/objects")?;
    let haves_set: std::collections::HashSet<String> = haves.iter().cloned().collect();

    let mut wants: Vec<String> = Vec::new();
    for (_branch, remote_sha) in &remote_heads {
        if !haves_set.contains(remote_sha) {
            wants.push(remote_sha.clone());
        }
    }

    if wants.is_empty() {
        // Also check if tracking refs are up to date
        let all_tracking_up_to_date = remote_heads.iter().all(|(branch, remote_sha)| {
            let tracking_path = format!(".crust/refs/remotes/{}/{}", remote_name, branch);
            std::fs::read_to_string(&tracking_path)
                .map(|s| s.trim() == remote_sha.as_str())
                .unwrap_or(false)
        });
        if all_tracking_up_to_date {
            println!("Already up to date.");
            return Ok(());
        }
        // We have the objects but tracking refs need updating — fall through
    }

    println!("remote: Enumerating objects: {} ref(s)", remote_heads.len());

    // Fetch objects in CRUSTPACK format (only if we have new wants)
    if !wants.is_empty() {
        let pack_data = sync.fetch(wants.clone(), haves)?;

        // Unpack and store objects locally
        let stored = unpack_and_store(".", &pack_data)?;

        println!("remote: Receiving objects: 100% ({}/{}), done.", stored, stored);
    }

    // Update remote tracking refs: .crust/refs/remotes/{remote}/{branch}
    for (branch, sha) in &remote_heads {
        let tracking_ref = format!(".crust/refs/remotes/{}/{}", remote_name, branch);
        if let Some(parent) = std::path::Path::new(&tracking_ref).parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(&tracking_ref, sha);
    }

    println!("Fetch complete.");

    Ok(())
}

/// Scan .crust/objects and collect all object IDs we already have.
fn collect_local_object_ids(objects_dir: &str) -> Result<Vec<String>> {
    let mut ids = Vec::new();
    let dir = Path::new(objects_dir);

    if !dir.exists() {
        return Ok(ids);
    }

    // Objects are stored as {objects_dir}/{xx}/{rest}
    for prefix_entry in fs::read_dir(dir)? {
        let prefix_entry = prefix_entry?;
        let prefix_path = prefix_entry.path();
        if prefix_path.is_dir() {
            let prefix = prefix_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();
            if prefix.len() == 2 {
                for obj_entry in fs::read_dir(&prefix_path)? {
                    let obj_entry = obj_entry?;
                    let suffix = obj_entry
                        .file_name()
                        .to_str()
                        .unwrap_or("")
                        .to_string();
                    if !suffix.is_empty() {
                        ids.push(format!("{}{}", prefix, suffix));
                    }
                }
            }
        }
    }

    Ok(ids)
}

/// Unpack CRUSTPACK data and write each object to .crust/objects/{xx}/{rest}.
/// The pack contains the raw (uncompressed) object bytes; we compress with zstd before storing.
pub fn unpack_and_store(repo_root: &str, pack_data: &[u8]) -> Result<usize> {
    let reader = PackReader::deserialize(pack_data)
        .map_err(|e| anyhow!("Failed to parse pack: {}", e))?;

    let objects = reader.into_objects();
    let mut stored = 0;

    for entry in &objects {
        let obj_id = &entry.id;
        if obj_id.len() < 3 {
            continue;
        }

        let obj_dir = format!("{}/.crust/objects/{}", repo_root, &obj_id[0..2]);
        fs::create_dir_all(&obj_dir)?;

        let obj_path = format!("{}/{}", obj_dir, &obj_id[2..]);

        // Skip if we already have it
        if Path::new(&obj_path).exists() {
            stored += 1;
            continue;
        }

        // Compress the raw object data with zstd before storing (matches local format)
        let compressed = zstd::encode_all(&entry.data[..], 3)
            .map_err(|e| anyhow!("Compression failed: {}", e))?;

        fs::write(&obj_path, &compressed)?;
        stored += 1;
    }

    Ok(stored)
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
