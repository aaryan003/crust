# CRUST CLI Client

A command-line interface for the CRUST distributed version control system. All commands start with `crust`.

## Installation

The CLI binary is built as part of the CRUST workspace:

```bash
cargo build --release
./target/release/crust --help
```

## Quick Start

### Initialize a repository

```bash
crust init
```

Creates a new `.crust/` directory with the repository structure.

### Authenticate

```bash
crust login https://your-crust-server.com
```

Stores credentials in `~/.crust/credentials` (JSON format).

### Work with files

```bash
crust add file.txt          # Stage a file
crust status               # Show working tree status
crust diff                 # Show unstaged changes
crust diff --staged        # Show staged changes
crust commit -m "Message"  # Create a commit
```

### Manage branches

```bash
crust branch               # List branches
crust branch feat/new      # Create a branch
crust checkout feat/new    # Switch branches
crust merge feat/new       # Merge a branch
```

### View history

```bash
crust log                  # Full commit history
crust log --oneline        # Compact history
crust show commit-id       # Show commit details
```

### Sync with remote

```bash
crust remote add origin https://server.com/user/repo
crust fetch                # Download objects from remote
crust push                 # Upload commits to remote
crust pull                 # Fetch and merge
crust clone <url> [dir]    # Clone a repository
```

## Debug Commands

The CLI includes several debug commands for troubleshooting:

### cat-object

Decompress and display an object's content (including header):

```bash
crust cat-object 5f87ad6a06fca8ea32d62365ea8bc2766bff7fedf62d6242db2884c25bf60cf1
```

Output:
```
CRUST-OBJECT
type: blob
size: 12

hello world
```

### hash-object

Compute the CRUST object ID for a file without storing it:

```bash
crust hash-object src/main.rs
```

Output:
```
3a7f8e9c1d2b4a6f5e3c1a9d7b5f3e1c2a4d6f8e9b1c3d5e7f9a0b2c4d6e8
```

### ls-tree

List the entries in a tree object:

```bash
crust ls-tree af109daab3401bb9be6580cc180548a22b861e6f42d4db65c27de520449e0e4d
```

Output:
```
100644 blob 5f87ad6a06fca8ea32d62365ea8bc2766bff7fedf62d6242db2884c25bf60cf1 file.txt
040000 tree abc12345... directory
```

### verify-pack

Validate the integrity of all objects in `.crust/objects/`:

```bash
crust verify-pack
```

Output:
```
Verifying 42 objects...
All objects OK
```

If corruption is detected:
```
Verifying 42 objects...
Found 1 corrupted objects:
  3a7f8e9c... — SHA256 mismatch: computed ..., expected ...
OBJECT_CORRUPT: 1 objects failed verification
```

## Configuration

Credentials are stored in `~/.crust/credentials`:

```json
{
  "servers": {
    "https://server.com": {
      "user_id": "alice",
      "token": "eyJ0eXAiOiJKV1QiLCJhbGc...",
      "expires_at": "2026-03-06T10:30:45Z"
    }
  },
  "remotes": {
    "origin": "https://server.com/alice/my-repo"
  }
}
```

Repositories use `.crust/config`:

```
[core]
    repositoryformatversion = 1
    currentBranch = main
```

## Exit Codes

- **0**: Success
- **1**: User error (bad arguments, missing repository, etc.)
- **2**: Runtime error (network error, disk error, server error)

## Error Codes

All errors are reported with error codes following CRUST conventions:

- `CLI_NO_REPOSITORY`: Not in a CRUST repository
- `CLI_INVALID_ARGUMENT`: Invalid command-line arguments
- `VALIDATE_INVALID_FORMAT`: Invalid input format
- `AUTH_INVALID_CREDENTIALS`: Authentication failed
- `OBJECT_NOT_FOUND`: Object doesn't exist
- `OBJECT_CORRUPT`: Object failed validation
- `REPO_NOT_FOUND`: Repository not found
- `REPO_PERMISSION_DENIED`: Insufficient permissions

See [contracts/error-codes.md](../contracts/error-codes.md) for the complete error code reference.

## Object Format

CRUST objects are stored in `.crust/objects/` with the following structure:

```
.crust/objects/{id[0..2]}/{id[2..64]}
```

Each object is:
1. Serialized with CRUST-OBJECT header
2. Compressed with zstd
3. Stored in binary format

The object format:

```
CRUST-OBJECT
type: {blob|tree|commit|tag}
size: {content_byte_length}

{raw content bytes}
```

The **Object ID** is the SHA256 hash of the entire serialized object (header + content).

## Cloning and Pushing

The CLI uses the CRUSTPACK wire protocol for efficient object transport:

```bash
crust clone https://server.com/user/repo
crust push origin main
```

CRUSTPACK format includes:
- Header with version and object count
- Serialized objects with size delimiters
- SHA256 trailer for integrity verification

## Troubleshooting

### "Not in a CRUST repository"

You're not inside a directory with a `.crust` folder. Run `crust init` first, or navigate to a CRUST repository.

### "Object not found"

The object ID doesn't exist. Try:
- `crust verify-pack` to check repository integrity
- `crust hash-object` to compute the correct ID

### "Object is corrupt"

The object failed validation. Try:
- `crust verify-pack` to see which objects are corrupted
- Re-cloning the repository if corruption is widespread

### "Could not reach server"

Check your network connection and the server URL. Verify the server is running with:

```bash
curl https://your-crust-server.com/health
```

## Development

To build the CLI for development:

```bash
cargo build -p crust-cli
./target/debug/crust --version
```

To run tests:

```bash
cargo test --workspace
```

To build for distribution (optimized):

```bash
cargo build --release -p crust-cli
strip target/release/crust  # Optional: reduce binary size
./target/release/crust --version
```

## Further Reading

- [CRUST Architecture](../docs/ARCHITECTURE.md)
- [Object Format Specification](../contracts/object-format.md)
- [CRUSTPACK Wire Protocol](../contracts/crustpack-format.md)
- [CLI Commands Specification](../contracts/cli-commands.md)
- [Error Codes Reference](../contracts/error-codes.md)
