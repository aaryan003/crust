// restore command - unstage files from the index or restore from HEAD

use crate::commands::checkout::{load_blob_content, load_tree_entries, load_tree_id_from_commit};
use crate::index::Index;
use crate::working_tree;
use anyhow::{anyhow, Result};
use std::fs;
use std::path::Path;

/// Restore a file.
/// - staged=true  → remove from index (unstage), don't touch working tree
/// - staged=false → restore file content from HEAD commit
pub fn cmd_restore(path: &str, staged: bool) -> Result<()> {
    let repo_root = ".";

    // Check if we're in a repo
    if !Path::new(".crust").exists() {
        return Err(anyhow!("CLI_NO_REPOSITORY: Not in a CRUST repository"));
    }

    if staged {
        // Unstage: remove from index only
        let mut index = Index::load(repo_root)?;
        if index.get_entry(path).is_none() {
            return Err(anyhow!("{}: not in index", path));
        }
        index.remove_entry(path);
        index.save(repo_root)?;
        println!("Unstaged {}", path);
    } else {
        // Restore working-tree file(s) from HEAD commit
        let head_ref = working_tree::get_head_ref(repo_root)?;
        let commit_id = match working_tree::read_ref(repo_root, &head_ref)? {
            Some(id) => id,
            None => return Err(anyhow!("No commits yet — nothing to restore from")),
        };
        let tree_id = load_tree_id_from_commit(repo_root, &commit_id)?;
        let entries: std::collections::HashMap<_, _> =
            load_tree_entries(repo_root, &tree_id)?.into_iter().collect();

        // "." means restore ALL tracked files from HEAD
        if path == "." {
            if entries.is_empty() {
                println!("Nothing to restore");
                return Ok(());
            }
            let mut count = 0;
            for (file_path, blob_id) in &entries {
                let content = load_blob_content(repo_root, blob_id)?;
                if let Some(parent) = Path::new(file_path).parent() {
                    if !parent.as_os_str().is_empty() {
                        fs::create_dir_all(parent)?;
                    }
                }
                fs::write(file_path, content)?;
                count += 1;
            }
            println!("Restored {} file(s)", count);
        } else {
            match entries.get(path) {
                Some(blob_id) => {
                    let content = load_blob_content(repo_root, blob_id)?;
                    if let Some(parent) = Path::new(path).parent() {
                        if !parent.as_os_str().is_empty() {
                            fs::create_dir_all(parent)?;
                        }
                    }
                    fs::write(path, content)?;
                    println!("Restored {}", path);
                }
                None => {
                    return Err(anyhow!(
                        "pathspec '{}' did not match any file known to CRUST",
                        path
                    ));
                }
            }
        }
    }

    Ok(())
}
