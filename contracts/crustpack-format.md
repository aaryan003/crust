# CRUSTPACK Wire Format Specification

VERSION: 1.0.0
WRITTEN_BY: contracts-agent
CONSUMED_BY: transport-agent, crust-cli
LAST_UPDATED: 2026-03-04

## Overview

CRUSTPACK is the format for transmitting CRUST objects over the network between client and server.

Used for:
- `POST /api/v1/repos/{owner}/{repo}/objects/upload` (client → server)
- `POST /api/v1/repos/{owner}/{repo}/objects/fetch` (server → client)

NOT the git packfile format. Human-readable headers, clean structure, flexible.

---

## Pack Structure

```
{pack header}
{per-object entries, count times}
{pack trailer}
```

---

## Pack Header

Literal ASCII text, CRLF or LF terminated lines.

```
CRUSTPACK\n
version: 1\n
count: {object_count}\n
\n
```

Fields:
- `CRUSTPACK\n`: Magic header (9 bytes + newline)
- `version: 1\n`: Protocol version (always "1" for v1)
- `count: {decimal_integer}\n`: How many objects follow
- Blank line: `\n` separates header from object section

Example:
```
CRUSTPACK
version: 1
count: 42

```

---

## Per-Object Entry

Each object entry (repeating `count` times):

```
id: {sha256_hex}\n
type: {blob|tree|commit|tag}\n
size: {compressed_byte_count}\n
{size bytes of zstd-compressed object data}
```

Fields:
- `id: {sha256_hex}\n`: 64-char lowercase hex SHA256
- `type: {type}\n`: Object type
- `size: {decimal_integer}\n`: Byte count of compressed data following
- Raw bytes: Exactly `size` bytes of zstd-compressed object (including header and content)

### Important Notes

1. **No delimiter between objects**: Use the `size` field to know where one object ends and next begins.
2. **Compression includes header**: The zstd data is the full object (header + content) compressed.
3. **Size is compressed size**: Not uncompressed size.
4. **Order**: No required order, but ascending by ID is conventional.

### Example Entry

Blob object:
- SHA256: `abc1234...` (truncated)
- Compressed to: 128 bytes

```
id: abc1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcd
type: blob
size: 128
[128 bytes of zstd data]
```

---

## Pack Trailer

After all object entries:

```
[32 bytes of raw SHA256 of all preceding pack bytes]
```

No header, no formatting. Exactly 32 raw bytes (not hex).

**Validation**: Compute SHA256 of all bytes from start of pack (including header) up to (but not including) the trailer. Compare with trailer bytes.

---

## Transmission Rules

### Upload (Client → Server)

1. Client builds CRUSTPACK with objects it needs to send.
2. POST the pack to `/api/v1/repos/{owner}/{repo}/objects/upload`
3. Body: raw bytes (Content-Type: `application/octet-stream`)
4. Server verifies trailer SHA256.
5. Server decompresses and validates each object.
6. Server stores validated objects to disk.
7. Response: `ObjectUploadResult` (JSON, wrapped in ApiResponse)

### Fetch (Server → Client)

1. Client POSTs to `/api/v1/repos/{owner}/{repo}/objects/fetch` with `RefPreflight` (list of wants).
2. Server looks up objects, compresses them into a CRUSTPACK.
3. Server returns pack as raw bytes.
4. Client verifies trailer SHA256.
5. Client decompresses and stores to `.crust/objects/`.

---

## Size Limits

### v1 Practical Limits

- **Single object**: No hard limit, but > 1 GB is not recommended (memory).
- **Pack size**: No hard limit, but consider connection timeout and memory.
- **Recommended**: Split uploads > 100 MB into multiple packs if possible.

Server may return HTTP 413 (Payload Too Large) if pack exceeds server limit.

---

## Example Full Pack

```
CRUSTPACK
version: 1
count: 2

id: abc1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcd
type: blob
size: 45
{45 bytes of zstd data for blob}
id: def4567890abcdef1234567890abcdef1234567890abcdef1234567890abcd123
type: tree
size: 87
{87 bytes of zstd data for tree}
{32 bytes of SHA256 trailer}
```

---

## Error Cases

### Invalid Pack Header
- Missing `CRUSTPACK` magic: Client/server returns `MALFORMED_PACK`
- Invalid `version:` field: Return `UNSUPPORTED_PACK_VERSION`
- Invalid `count:`: Return `MALFORMED_PACK`

### Missing/Corrupt Object
- Expected `count` objects but fewer bytes available: `TRUNCATED_PACK`
- Trailer SHA256 doesn't match: `PACK_CHECKSUM_MISMATCH`

### Invalid Compressed Data
- zstd decompression fails: `CORRUPT_OBJECT`

### Invalid Object Format
- Decompressed object header malformed: `INVALID_OBJECT_HEADER`
- Object size in header doesn't match actual: `INVALID_OBJECT_SIZE`

---

## Backwards Compatibility

Version 1 is the initial release. Future versions will increment the `version:` field.

Clients/servers must reject packs with unsupported versions rather than guessing.

---

## Implementation Notes

### Writing a Pack

```pseudocode
function write_pack(objects: List<Object>) -> ByteBuffer {
    buf = ByteBuffer()
    
    buf.write_text("CRUSTPACK\n")
    buf.write_text("version: 1\n")
    buf.write_text(f"count: {len(objects)}\n")
    buf.write_text("\n")
    
    for obj in objects {
        compressed = zstd_compress(obj.get_full_bytes())
        buf.write_text(f"id: {obj.sha256_hex()}\n")
        buf.write_text(f"type: {obj.type()}\n")
        buf.write_text(f"size: {len(compressed)}\n")
        buf.write_bytes(compressed)
    }
    
    pack_checksum = sha256(buf.get_all_bytes())
    buf.write_bytes(pack_checksum)
    
    return buf
}
```

### Reading a Pack

```pseudocode
function read_pack(data: ByteBuffer) -> List<Object> {
    pos = 0
    
    header_text = read_until_blank_line(data)
    version = parse_line(header_text, "version:")  // expect "1"
    count = parse_line(header_text, "count:")
    
    pos = find_blank_line(data) + 2  // skip blank line
    
    objects = []
    for i in range(count) {
        id = read_line_field(data, "id:", pos)
        type = read_line_field(data, "type:", pos)
        size = read_line_field(data, "size:", pos)
        
        compressed_bytes = data[pos : pos + size]
        pos += size
        
        full_bytes = zstd_decompress(compressed_bytes)
        obj = parse_object(full_bytes)
        objects.append(obj)
    }
    
    pack_checksum_on_disk = data[pos : pos + 32]
    expected_checksum = sha256(data[0 : pos])
    
    if pack_checksum_on_disk != expected_checksum {
        raise PackChecksumMismatch
    }
    
    return objects
}
```

---

## Transport Notes

### HTTP Headers

When uploading:
```
POST /api/v1/repos/owner/repo/objects/upload
Content-Type: application/octet-stream
Content-Length: {pack_byte_size}
Authorization: Bearer {jwt}
```

When downloading:
```
HTTP/1.1 200 OK
Content-Type: application/octet-stream
Content-Length: {pack_byte_size}
```

### Connection Handling

Packs can be large. Server should support:
- Streaming upload (chunked transfer encoding)
- HTTP/1.1 or HTTP/2
- Timeout: >= 5 minutes for large packs

Client should:
- Stream to disk during download to avoid memory exhaustion
- Show progress bar (using pack's `count` field)
- Retry on network failure (idempotent: re-upload same pack)

---

## Security

- HTTPS required: All pack transmission encrypted.
- JWT required: Client must authenticate before uploading/downloading.
- Server validates: After unpacking, verify all object formats are correct before storing.
- Rate limiting: Server may limit pack size or upload frequency per user/repo.
