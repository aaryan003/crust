//! CRUST command-line client

use clap::{Parser, Subcommand};
use std::process;

mod client;
mod commands;
mod config;
mod index;
mod pack;
mod refs;
mod remote;
mod working_tree;

#[derive(Parser)]
#[command(name = "crust")]
#[command(about = "CRUST — A modern version control system", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new CRUST repository
    Init {
        /// Optional directory name (defaults to current directory)
        #[arg(value_name = "DIRECTORY")]
        directory: Option<String>,
    },

    /// Authenticate with a CRUST server
    Login {
        /// Server URL
        server: String,
        /// Username (non-interactive)
        #[arg(long, value_name = "USERNAME")]
        username: Option<String>,
        /// Password (non-interactive; empty string is rejected)
        #[arg(long, value_name = "PASSWORD")]
        password: Option<String>,
    },

    /// Logout from a CRUST server
    Logout {
        /// Server URL (optional, uses default if only one is configured)
        server: Option<String>,
    },

    /// Display current authenticated user
    Whoami {
        /// Server URL (optional, uses default if only one is configured)
        server: Option<String>,
    },

    /// Display current working tree status
    Status,

    /// Stage file(s) in the index
    Add {
        /// File or directory path(s) (use "." for all)
        #[arg(required = true, num_args = 1..)]
        paths: Vec<String>,
    },

    /// Unstage file(s) from the index
    Restore {
        /// File or directory path
        path: String,

        /// Unstage a file (remove from index)
        #[arg(long)]
        staged: bool,
    },

    /// Show unstaged changes
    Diff {
        /// Show staged changes instead (--staged)
        #[arg(long)]
        staged: bool,

        /// Additional arguments: file path, or two commit refs
        #[arg(value_name = "ARGS", num_args = 0..=2)]
        args: Vec<String>,
    },

    /// Show commit history
    Log {
        /// Show commits in compact format
        #[arg(long)]
        oneline: bool,

        /// Limit number of commits shown
        #[arg(short = 'n', long = "max-count", value_name = "NUMBER")]
        max_count: Option<usize>,

        /// Show log for a specific branch or ref
        #[arg(value_name = "BRANCH")]
        branch: Option<String>,
    },

    /// Show commit details and diff
    Show {
        /// Branch name or commit ID
        #[arg(value_name = "REF")]
        reference: String,
    },

    /// List, create, or delete branches
    Branch {
        /// Branch name to create (optional)
        #[arg(value_name = "BRANCH")]
        branch: Option<String>,

        /// Delete branch (safe: checks if merged)
        #[arg(short)]
        delete: bool,

        /// Force delete branch (even if not merged)
        #[arg(short = 'D')]
        force_delete: bool,

        /// Show branch SHA and commit message
        #[arg(short = 'v', long)]
        verbose: bool,
    },

    /// Switch to a branch (or checkout file from another branch)
    Checkout {
        /// Branch name or commit SHA
        #[arg(value_name = "BRANCH")]
        branch: String,

        /// Create new branch before switching (-b)
        #[arg(short = 'b')]
        create: bool,

        /// Files to checkout from the specified branch (-- file1 file2 ...)
        #[arg(last = true, value_name = "FILES")]
        files: Vec<String>,
    },

    /// Merge another branch into current branch
    Merge {
        /// Branch to merge (required unless --abort is used)
        #[arg(value_name = "BRANCH", required_unless_present = "abort")]
        branch: Option<String>,

        /// Abort an in-progress merge
        #[arg(long)]
        abort: bool,
    },

    /// Create a new commit
    Commit {
        /// Commit message
        #[arg(short, long)]
        message: Option<String>,
    },

    /// Manage remote repositories
    Remote {
        #[command(subcommand)]
        action: RemoteAction,
    },

    /// Fetch objects from remote
    Fetch {
        /// Remote name (default: origin)
        #[arg(value_name = "REMOTE")]
        remote: Option<String>,

        /// Specific branch to fetch (optional)
        #[arg(value_name = "BRANCH")]
        branch: Option<String>,
    },

    /// Push commits to remote
    Push {
        /// Remote name (default: origin)
        #[arg(value_name = "REMOTE")]
        remote: Option<String>,

        /// Branch name (default: current branch)
        #[arg(value_name = "BRANCH")]
        branch: Option<String>,

        /// Set upstream tracking
        #[arg(short = 'u', long = "set-upstream")]
        set_upstream: bool,

        /// Force push (overwrite remote history)
        #[arg(long)]
        force: bool,
    },

    /// Fetch from remote and merge
    Pull {
        /// Remote name (default: origin)
        #[arg(value_name = "REMOTE")]
        remote: Option<String>,

        /// Branch name (default: current branch)
        #[arg(value_name = "BRANCH")]
        branch: Option<String>,

        /// Rebase instead of merge
        #[arg(long)]
        rebase: bool,
    },

    /// Clone a repository
    Clone {
        /// Repository URL
        url: String,

        /// Directory to clone into (default: repo name)
        #[arg(value_name = "DIRECTORY")]
        directory: Option<String>,
    },

    /// Decompress and print object content
    CatObject {
        /// Object ID (SHA256 hash)
        #[arg(value_name = "ID")]
        id: String,

        /// Show the object type
        #[arg(short = 't', long = "type")]
        show_type: bool,

        /// Show the object size
        #[arg(short = 's', long = "size")]
        show_size: bool,
    },

    /// Compute object ID for a file
    HashObject {
        /// File path (omit when using --stdin)
        #[arg(value_name = "FILE")]
        file: Option<String>,

        /// Write object to store
        #[arg(short = 'w', long = "write")]
        write_obj: bool,

        /// Read from stdin instead of a file
        #[arg(long = "stdin")]
        from_stdin: bool,
    },

    /// List tree entries
    LsTree {
        /// Tree object ID, branch name, or HEAD
        #[arg(value_name = "ID")]
        id: String,

        /// List recursively
        #[arg(short = 'r', long = "recursive")]
        recursive: bool,

        /// Show only filenames (no metadata)
        #[arg(long = "name-only")]
        name_only: bool,

        /// Filter to a subdirectory path
        #[arg(value_name = "PATH")]
        path_filter: Option<String>,
    },

    /// Verify object storage integrity
    VerifyPack {
        /// Pack file path (optional, for compatibility)
        #[arg(value_name = "PATH")]
        path: Option<String>,

        /// Verbose: list each object with type and size
        #[arg(short = 'v', long)]
        verbose: bool,
    },
<<<<<<< HEAD
=======

    /// Resolve a reference and print its commit ID
    RevParse {
        /// Reference to resolve (HEAD, branch name, or commit SHA)
        #[arg(value_name = "REF")]
        reference: String,
    },
>>>>>>> upstream/main
}

#[derive(Subcommand)]
enum RemoteAction {
    /// Add a new remote
    Add {
        /// Remote name
        name: String,

        /// Remote URL
        url: String,
    },

    /// List all remotes
    List,

    /// Remove a remote
    Remove {
        /// Remote name
        name: String,
    },

    /// Rename a remote
    Rename {
        /// Current remote name
        old_name: String,
        /// New remote name
        new_name: String,
    },

    /// Update remote URL
    SetUrl {
        /// Remote name
        name: String,
        /// New URL
        url: String,
    },
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Some(Commands::Init { directory }) => commands::cmd_init(directory.as_deref()),

        Some(Commands::Login { server, username, password }) => commands::cmd_login(&server, username.as_deref(), password.as_deref()),

        Some(Commands::Logout { server }) => commands::cmd_logout(server.as_deref()),

        Some(Commands::Whoami { server }) => commands::cmd_whoami(server.as_deref()),

        Some(Commands::Status) => commands::cmd_status(),

        Some(Commands::Add { paths }) => commands::cmd_add(&paths),

        Some(Commands::Restore { path, staged }) => commands::cmd_restore(&path, staged),

        Some(Commands::Diff { staged, args }) => commands::cmd_diff(staged, &args),

        Some(Commands::Log { oneline, max_count, branch }) => {
            if oneline {
                commands::cmd_log_oneline(branch.as_deref(), max_count)
            } else {
                commands::cmd_log(branch.as_deref(), max_count)
            }
        }

        Some(Commands::Show { reference }) => commands::cmd_show(&reference),

        Some(Commands::Branch { branch, delete, force_delete, verbose }) => {
            commands::cmd_branch(branch.as_deref(), branch.as_deref(), delete, force_delete, verbose)
        }

        Some(Commands::Checkout { branch, create, files }) => {
            if !files.is_empty() {
                commands::cmd_checkout_files(&branch, &files)
            } else {
                commands::cmd_checkout(&branch, create)
            }
        }

        Some(Commands::Merge { branch, abort }) => {
            if abort {
                commands::merge::cmd_merge_abort()
            } else {
                commands::cmd_merge(branch.as_deref().unwrap())
            }
        }

        Some(Commands::Commit { message }) => commands::cmd_commit(message.as_deref()),

        Some(Commands::Remote { action }) => match action {
            RemoteAction::Add { name, url } => commands::cmd_remote_add(&name, &url),
            RemoteAction::List => commands::cmd_remote_list(),
            RemoteAction::Remove { name } => commands::cmd_remote_remove(&name),
            RemoteAction::Rename { old_name, new_name } => commands::cmd_remote_rename(&old_name, &new_name),
            RemoteAction::SetUrl { name, url } => commands::cmd_remote_set_url(&name, &url),
        },

        Some(Commands::Fetch { remote, branch }) => commands::cmd_fetch(remote.as_deref(), branch.as_deref()),

        Some(Commands::Push { remote, branch, force, set_upstream }) => {
            commands::cmd_push(remote.as_deref(), branch.as_deref(), force, set_upstream)
        }

        Some(Commands::Pull { remote, branch, rebase }) => {
            commands::cmd_pull(remote.as_deref(), branch.as_deref(), rebase)
        }

        Some(Commands::Clone { url, directory }) => commands::cmd_clone(&url, directory.as_deref()),

        Some(Commands::CatObject { id, show_type, show_size }) => commands::cmd_cat_object(&id, show_type, show_size),

        Some(Commands::HashObject { file, write_obj, from_stdin }) => commands::cmd_hash_object(file.as_deref(), write_obj, from_stdin),

        Some(Commands::LsTree { id, recursive, name_only, path_filter }) => commands::cmd_ls_tree(&id, recursive, name_only, path_filter.as_deref()),

        Some(Commands::VerifyPack { path, verbose }) => commands::cmd_verify_pack(verbose, path.as_deref()),

<<<<<<< HEAD
=======
        Some(Commands::RevParse { reference }) => commands::cmd_rev_parse(&reference),

>>>>>>> upstream/main
        None => {
            println!("Use 'crust --help' for usage information");
            Ok(())
        }
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}
