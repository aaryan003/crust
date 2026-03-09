# CRUST Object Format Specification

VERSION: 1.0.0
WRITTEN_BY: contracts-agent
CONSUMED_BY: gitcore, crust-server, crust-cli
LAST_UPDATED: 2026-03-04

## Overview
CRUST uses SHA256-identified, zstd-compressed objects. This is NOT git format. It is completely original.

---

## Object ID (SHA256)

- **Algorithm**: SHA256 (256-bit hash)
- **Format**: 64 lowercase hexadecimal characters
- **Computed over**: The full object bytes in memory before compression (header + content concatenated)
- **Immutable**: Once computed, the ID never changes

Example: `3a7f8e9c1d2b4a6f5e3c1a9d7b5f3e1c2a4d6f8e9b1c3d5e7f9a0b2c4d6e8`

---

## Object Storage (Loose Format)

### Path
```
.crust/objects/{id[0..2]}/{id[2..64]}
```

Example with ID above:
```
.crust/objects/3a/7f8e9c1d2b4a6f5e3c1a9d7b5f3e1c2a4d6f8e9b1c3d5e7f9a0b2c4d6e8
```

### Compression
- Content on disk: zstd-compressed object bytes
- Compression level: 3 (default balance)
- Decompression: gunzip/zstd can read directly

---

## Object Header Format

Before content bytes are hashed and stored, they are prefixed with a header in text format.

**Text header followed by binary content, all together hashed as SHA256.**

```
CRUST-OBJECT\n
type: {blob|tree|commit|tag}\n
size: {raw_content_byte_length}\n
\n
{raw content bytes follow immediately}
```

Rules:
- `CRUST-OBJECT\n` is literal (11 bytes + newline)
- `type: ` is literal (6 bytes including space)
- `size: ` is literal (6 bytes including space)
- Object type is one of: blob, tree, commit, tag
- Size is decimal integer representing uncompressed content byte length (before zstd)
- Blank line: `\n\n` separates header from content
- Content follows immediately with no additional framing

### Example Header

For a blob with 42 bytes of content:
```
CRUST-OBJECT
type: blob
size: 42

{42 bytes of file content}
```

---

## Blob Objects

**Content**: Raw file bytes, no transformation, no escaping.

Stores exactly what the file contains on disk.

### Example
File: `src/main.rs` containing 1024 bytes of Rust code

Object content section is those 1024 bytes exactly.

---

## Tree Objects

**Content**: Binary format representing directory structure.

Each tree entry is:
```
{mode_ascii_decimal} {name_utf8}\0{32_raw_sha256_bytes}
```

Fields:
- `mode_ascii_decimal`: Space-padded to 6 characters, e.g., `100644`, `100755`, `040000`, `120000`
- Space: literal ASCII space (0x20)
- `name_utf8`: File or directory name in UTF-8 (no slashes for dirs, just the name)
- `\0`: Null byte (0x00)
- `sha256_bytes`: Raw 32 bytes (not hex) of the object's SHA256

### Sorting
Tree entries are **sorted lexicographically by name**, with directories sorting as if they have a trailing slash.

Example order:
```
.gitignore          (if it existed)
README.md
bin/                (sorts after dir name without /)
src/                (but before "src_utils/")
src_utils.txt
```

### Modes
| Mode | Type |
|------|------|
| 100644 | Regular file |
| 100755 | Executable file |
| 040000 | Directory (tree) |
| 120000 | Symlink |

### Binary Encoding Example

Tree with two entries: `README.md` (blob SHA256: `abc...`) and `src/` (tree SHA256: `def...`)

Hex dump:
```
31 30 30 36 34 34 20 52 45 41 44 4d 45 2e 6d 64 00 [32 bytes of abc...]
34 30 30 30 30 20 73 72 63 00 [32 bytes of def...]
```

Breaking down:
- `31 30 30 36 34 34` = "100644" (ASCII)
- `20` = space
- `52 45 41 44 4d 45 2e 6d 64` = "README.md" (UTF-8)
- `00` = null byte
- Next 32 bytes: raw SHA256 of `README.md` blob
- Repeat for `src/` entry

---

## Commit Objects

**Content**: Text format (UTF-8) representing a point in history.

```
tree {sha256_hex}\n
parent {sha256_hex}\n        [one or more for merge commits; none for root]
author {name} <{email}> {unix_timestamp} {tz_offset}\n
committer {name} <{email}> {unix_timestamp} {tz_offset}\n
\n
{commit message — rest of file}
```

Fields:
- `tree`: SHA256 (hex) of the root tree object
- `parent`: One line per parent (0 for root commit, 1 for normal, 2+ for merge)
- `author`: Person who wrote the change
- `committer`: Person who applied the change (usually same as author)
- `unix_timestamp`: Seconds since epoch (0 is 1970-01-01 00:00:00 UTC)
- `tz_offset`: E.g., `+0000` (UTC), `-0500` (EST), `+0530` (IST)
- Blank line separates metadata from message
- Message: Everything after blank line, no length limit, preserves newlines

### Example Commit

```
tree 3a7f8e9c1d2b4a6f5e3c1a9d7b5f3e1c2a4d6f8e9b1c3d5e7f9a0b2c4d6e8
parent 1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f0a1
author Jane Smith <jane@example.com> 1704067200 +0000
committer Jane Smith <jane@example.com> 1704067200 +0000

Add authentication system

This commit implements JWT-based auth for the platform.
It includes registration, login, and token refresh endpoints.

Signed-off-by: Jane Smith <jane@example.com>
```

### Root Commit
No `parent` lines.

### Merge Commit
Multiple `parent` lines (one per merged branch).

---

## Tag Objects

**Content**: Text format (UTF-8) for annotated tags.

```
object {sha256_hex}\n
type {blob|tree|commit|tag}\n
tag {tag_name}\n
tagger {name} <{email}> {unix_timestamp} {tz_offset}\n
\n
{tag message}
```

Fields:
- `object`: SHA256 (hex) of the tagged object
- `type`: Object type being tagged (usually `commit`)
- `tag`: Tag name (e.g., `v0.1.0`, `release/beta-1`)
- `tagger`: Person who created the tag
- `unix_timestamp`: When tag was created
- `tz_offset`: Timezone of tagger
- Message: Tag annotation (release notes, changelog, etc.)

### Example Tag

```
object 3a7f8e9c1d2b4a6f5e3c1a9d7b5f3e1c2a4d6f8e9b1c3d5e7f9a0b2c4d6e8
type commit
tag v0.1.0
tagger Release Bot <release@crust.dev> 1704067200 +0000

Version 0.1.0 - Initial Release

This is the first public release of CRUST.

Features:
- Object storage with SHA256
- Basic VCS operations (init, commit, push, pull)
```

---

## Validation

When reading an object:
1. Decompress using zstd
2. Parse header (expect exact format)
3. Read content of specified size
4. Recompute SHA256(header + content)
5. Verify matches the object ID (filename)
6. If mismatch: corrupt object, raise error

---

## Edge Cases

### Empty Blob
Size: 0
Content: zero bytes
SHA256 computed on header only: `CRUST-OBJECT\ntype: blob\nsize: 0\n\n`

### Empty Tree
Size: 0
Content: zero bytes
Represents an empty directory (rare but valid)

### Very Large Objects
No size limit in format. Client/server may impose practical limits (e.g., 1GB max).

---

## Not in This Format

- **No delta compression**: CRUSTPACK (v1) sends full objects. Delta is future work.
- **No streaming**: Full object content must fit in memory to hash.
- **No signatures**: Objects are signed at transport level (HTTPS + JWT), not embedded.
- **No metadata**: Modification time, permissions, ownership are not stored in object.
