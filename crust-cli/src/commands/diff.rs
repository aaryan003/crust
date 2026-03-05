// diff command - show changes between working tree, index, and commits

use crate::commands::checkout::{load_blob_content, load_tree_entries, load_tree_id_from_commit};
use crate::index::Index;
use crate::working_tree;
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Entry point for crust diff [--staged] [args...]
/// args = []           → unstaged (working tree vs HEAD)
/// args = []  --staged → staged (index vs HEAD)
/// args = [file]       → unstaged diff for that specific file
/// args = [ref1 ref2]  → diff between two commits or branches
pub fn cmd_diff(staged: bool, args: &[String]) -> Result<()> {
    if !Path::new(".crust").exists() {
        return Err(anyhow!("CLI_NO_REPOSITORY: Not in a CRUST repository"));
    }
    let repo_root = ".";
    match args.len() {
        0 => {
            if staged {
                diff_staged(repo_root)
            } else {
                diff_unstaged(repo_root, None)
            }
        }
        1 => {
            let arg = &args[0];
            if Path::new(arg).exists() {
                diff_unstaged(repo_root, Some(arg.as_str()))
            } else {
                let head_commit = get_head_commit(repo_root)?;
                let other_commit = resolve_ref_to_commit(repo_root, arg)?;
                diff_commits(repo_root, &head_commit, &other_commit)
            }
        }
        2 => {
            let commit_a = resolve_ref_to_commit(repo_root, &args[0])?;
            let commit_b = resolve_ref_to_commit(repo_root, &args[1])?;
            diff_commits(repo_root, &commit_a, &commit_b)
        }
        _ => Err(anyhow!("Too many arguments for diff")),
    }
}

fn get_head_commit(repo_root: &str) -> Result<String> {
    let head_ref = working_tree::get_head_ref(repo_root)?;
    working_tree::read_ref(repo_root, &head_ref)?
        .ok_or_else(|| anyhow!("HEAD has no commits yet"))
}

fn resolve_ref_to_commit(repo_root: &str, refname: &str) -> Result<String> {
    // Try as branch
    if let Ok(Some(sha)) = working_tree::read_ref(repo_root, &format!("refs/heads/{}", refname)) {
        return Ok(sha);
    }
    // Try as remote tracking ref (e.g. origin/main)
    if refname.contains('/') {
        if let Ok(Some(sha)) =
            working_tree::read_ref(repo_root, &format!("refs/remotes/{}", refname))
        {
            return Ok(sha);
        }
    }
    // Try as raw SHA256
    if refname.len() == 64 && refname.chars().all(|c| c.is_ascii_hexdigit()) {
        let obj_path = format!(
            "{}/.crust/objects/{}/{}",
            repo_root,
            &refname[0..2],
            &refname[2..]
        );
        if Path::new(&obj_path).exists() {
            return Ok(refname.to_string());
        }
    }
    Err(anyhow!("Unknown ref or path: '{}'", refname))
}

fn load_head_tree_map(repo_root: &str) -> Result<HashMap<String, Vec<u8>>> {
    let head_ref = working_tree::get_head_ref(repo_root)?;
    match working_tree::read_ref(repo_root, &head_ref)? {
        None => Ok(HashMap::new()),
        Some(commit_id) => load_commit_tree_map(repo_root, &commit_id),
    }
}

fn load_commit_tree_map(repo_root: &str, commit_id: &str) -> Result<HashMap<String, Vec<u8>>> {
    let tree_id = load_tree_id_from_commit(repo_root, commit_id)?;
    let entries = load_tree_entries(repo_root, &tree_id)?;
    let mut map = HashMap::new();
    for (path, blob_id) in entries {
        let content = load_blob_content(repo_root, &blob_id)?;
        map.insert(path, content);
    }
    Ok(map)
}

/// Unstaged diff: working tree vs HEAD
fn diff_unstaged(repo_root: &str, file_filter: Option<&str>) -> Result<()> {
    let head_map = load_head_tree_map(repo_root)?;
    let working_files = working_tree::scan_working_tree(repo_root, Some("."))?;
    let working_map: HashMap<String, String> = working_files
        .iter()
        .map(|f| (f.path.clone(), f.path.clone()))
        .collect();

    let mut has_changes = false;
    let mut paths: Vec<&String> = head_map.keys().collect();
    paths.sort();

    for path in paths {
        if let Some(filter) = file_filter {
            if path != filter {
                continue;
            }
        }
        let head_content = &head_map[path];
        match working_map.get(path) {
            None => {
                // File deleted from working tree
                println!("diff --crust {}", path);
                println!("deleted file mode 100644");
                println!("--- a/{}", path);
                println!("+++ /dev/null");
                let old_str = String::from_utf8_lossy(head_content);
                for line in old_str.lines() {
                    println!("-{}", line);
                }
                has_changes = true;
            }
            Some(wt_path) => {
                let wt_content = fs::read(wt_path).unwrap_or_default();
                if &wt_content != head_content {
                    print_unified_diff(path, head_content, &wt_content, "a", "b");
                    has_changes = true;
                }
            }
        }
    }

    if !has_changes {
        if let Some(f) = file_filter {
            println!("No changes to '{}'", f);
        } else {
            println!("No unstaged changes");
        }
    }
    Ok(())
}

/// Staged diff: index vs HEAD
fn diff_staged(repo_root: &str) -> Result<()> {
    let index = Index::load(repo_root)?;
    let head_map = load_head_tree_map(repo_root)?;
    let mut has_changes = false;

    let mut entries: Vec<_> = index.entries().iter().cloned().collect();
    entries.sort_by(|a, b| a.path.cmp(&b.path));

    for entry in &entries {
        let index_content = load_blob_content(repo_root, &entry.blob_id)?;
        match head_map.get(&entry.path) {
            None => {
                println!("diff --crust {}", entry.path);
                println!("new file mode 100644");
                println!("--- /dev/null");
                println!("+++ b/{}", entry.path);
                let s = String::from_utf8_lossy(&index_content);
                for line in s.lines() {
                    println!("+{}", line);
                }
                has_changes = true;
            }
            Some(head_content) => {
                if &index_content != head_content {
                    print_unified_diff(&entry.path, head_content, &index_content, "a", "b");
                    has_changes = true;
                }
            }
        }
    }

    // Files deleted from index (in HEAD but not staged)
    let mut deleted_paths: Vec<&String> = head_map
        .keys()
        .filter(|p| index.get_entry(p).is_none())
        .collect();
    deleted_paths.sort();
    for path in deleted_paths {
        let head_content = &head_map[path];
        println!("diff --crust {}", path);
        println!("deleted file mode 100644");
        println!("--- a/{}", path);
        println!("+++ /dev/null");
        let s = String::from_utf8_lossy(head_content);
        for line in s.lines() {
            println!("-{}", line);
        }
        has_changes = true;
    }

    if !has_changes {
        println!("No staged changes");
    }
    Ok(())
}

/// Diff between two commits/branches
fn diff_commits(repo_root: &str, commit_a: &str, commit_b: &str) -> Result<()> {
    let map_a = load_commit_tree_map(repo_root, commit_a)?;
    let map_b = load_commit_tree_map(repo_root, commit_b)?;

    let mut all_paths: std::collections::HashSet<String> = std::collections::HashSet::new();
    all_paths.extend(map_a.keys().cloned());
    all_paths.extend(map_b.keys().cloned());
    let mut paths: Vec<String> = all_paths.into_iter().collect();
    paths.sort();

    let mut has_changes = false;
    for path in &paths {
        match (map_a.get(path), map_b.get(path)) {
            (Some(ca), Some(cb)) => {
                if ca != cb {
                    print_unified_diff(path, ca, cb, "a", "b");
                    has_changes = true;
                }
            }
            (None, Some(cb)) => {
                println!("diff --crust {}", path);
                println!("new file mode 100644");
                println!("--- /dev/null");
                println!("+++ b/{}", path);
                let s = String::from_utf8_lossy(cb);
                for line in s.lines() {
                    println!("+{}", line);
                }
                has_changes = true;
            }
            (Some(ca), None) => {
                println!("diff --crust {}", path);
                println!("deleted file mode 100644");
                println!("--- a/{}", path);
                println!("+++ /dev/null");
                let s = String::from_utf8_lossy(ca);
                for line in s.lines() {
                    println!("-{}", line);
                }
                has_changes = true;
            }
            (None, None) => {}
        }
    }
    if !has_changes {
        println!("No differences");
    }
    Ok(())
}

/// Print a unified diff with proper +/- markers and hunk headers
fn print_unified_diff(path: &str, old: &[u8], new: &[u8], old_prefix: &str, new_prefix: &str) {
    // Binary file detection
    if old.iter().any(|&b| b == 0) || new.iter().any(|&b| b == 0) {
        println!("diff --crust {}", path);
        println!(
            "Binary files {}/{} and {}/{} differ",
            old_prefix, path, new_prefix, path
        );
        return;
    }
    let old_str = String::from_utf8_lossy(old);
    let new_str = String::from_utf8_lossy(new);
    let old_lines: Vec<&str> = old_str.lines().collect();
    let new_lines: Vec<&str> = new_str.lines().collect();

    let hunks = compute_hunks(&old_lines, &new_lines);
    if hunks.is_empty() {
        return;
    }
    println!("diff --crust {}", path);
    println!("--- {}/{}", old_prefix, path);
    println!("+++ {}/{}", new_prefix, path);
    for hunk in hunks {
        println!("{}", hunk.header);
        for line in hunk.lines {
            println!("{}", line);
        }
    }
}

struct Hunk {
    header: String,
    lines: Vec<String>,
}

const CONTEXT: usize = 3;

fn compute_hunks(old: &[&str], new: &[&str]) -> Vec<Hunk> {
    let edits = lcs_diff(old, new);
    if !edits.iter().any(|(op, _)| *op != ' ') {
        return vec![];
    }

    // Assign old/new line numbers to each edit op
    let mut old_line = 1usize;
    let mut new_line = 1usize;
    let mut numbered: Vec<(char, &str, usize, usize)> = Vec::new();
    for (op, line) in &edits {
        match *op {
            '+' => {
                numbered.push(('+', line, 0, new_line));
                new_line += 1;
            }
            '-' => {
                numbered.push(('-', line, old_line, 0));
                old_line += 1;
            }
            _ => {
                numbered.push((' ', line, old_line, new_line));
                old_line += 1;
                new_line += 1;
            }
        }
    }

    let change_positions: Vec<usize> = numbered
        .iter()
        .enumerate()
        .filter(|(_, (op, _, _, _))| *op != ' ')
        .map(|(i, _)| i)
        .collect();

    if change_positions.is_empty() {
        return vec![];
    }

    let mut hunks = Vec::new();
    let mut ci = 0;
    while ci < change_positions.len() {
        let start = change_positions[ci].saturating_sub(CONTEXT);
        let mut end = change_positions[ci] + CONTEXT + 1;

        while ci + 1 < change_positions.len()
            && change_positions[ci + 1] <= end + CONTEXT
        {
            ci += 1;
            end = change_positions[ci] + CONTEXT + 1;
        }
        end = end.min(numbered.len());

        let hunk_slice = &numbered[start..end];

        let old_start = hunk_slice
            .iter()
            .find(|(op, _, old, _)| *op != '+' && *old > 0)
            .map(|(_, _, old, _)| *old)
            .unwrap_or(1);
        let new_start = hunk_slice
            .iter()
            .find(|(op, _, _, new)| *op != '-' && *new > 0)
            .map(|(_, _, _, new)| *new)
            .unwrap_or(1);
        let old_count = hunk_slice.iter().filter(|(op, _, _, _)| *op != '+').count();
        let new_count = hunk_slice.iter().filter(|(op, _, _, _)| *op != '-').count();

        let header = format!(
            "@@ -{},{} +{},{} @@",
            old_start, old_count, new_start, new_count
        );
        let lines: Vec<String> = hunk_slice
            .iter()
            .map(|(op, line, _, _)| format!("{}{}", op, line))
            .collect();

        hunks.push(Hunk { header, lines });
        ci += 1;
    }
    hunks
}

/// LCS-based diff returning (op, line) where op is ' ', '+', or '-'
fn lcs_diff<'a>(old: &[&'a str], new: &[&'a str]) -> Vec<(char, &'a str)> {
    let m = old.len();
    let n = new.len();

    // Guard against huge O(m*n) allocations
    if m * n > 500_000 {
        let mut result = Vec::new();
        for line in old {
            result.push(('-', *line));
        }
        for line in new {
            result.push(('+', *line));
        }
        return result;
    }

    let mut dp = vec![vec![0u32; n + 1]; m + 1];
    for i in 1..=m {
        for j in 1..=n {
            if old[i - 1] == new[j - 1] {
                dp[i][j] = dp[i - 1][j - 1] + 1;
            } else {
                dp[i][j] = dp[i - 1][j].max(dp[i][j - 1]);
            }
        }
    }

    let mut ops = Vec::with_capacity(m + n);
    let mut i = m;
    let mut j = n;
    while i > 0 || j > 0 {
        if i > 0 && j > 0 && old[i - 1] == new[j - 1] {
            ops.push((' ', old[i - 1]));
            i -= 1;
            j -= 1;
        } else if j > 0 && (i == 0 || dp[i][j - 1] >= dp[i - 1][j]) {
            ops.push(('+', new[j - 1]));
            j -= 1;
        } else {
            ops.push(('-', old[i - 1]));
            i -= 1;
        }
    }
    ops.reverse();
    ops
}
