// CRUSTPACK format handling for wire protocol

use anyhow::{anyhow, Result};
use sha2::{Digest, Sha256};

/// Pack header fields
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct PackHeader {
    pub version: u32,
    pub count: usize,
}

/// Per-object entry in pack
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ObjectEntry {
    pub id: String,
    pub object_type: String,
    pub size: usize,
    pub data: Vec<u8>,
}

/// CRUSTPACK writer - builds pack format
#[allow(dead_code)]
pub struct PackWriter {
    header: PackHeader,
    objects: Vec<ObjectEntry>,
}

impl PackWriter {
    pub fn new() -> Self {
        PackWriter {
            header: PackHeader {
                version: 1,
                count: 0,
            },
            objects: Vec::new(),
        }
    }

    #[allow(dead_code)]
    pub fn add_object(&mut self, id: String, object_type: String, data: Vec<u8>) -> Result<()> {
        let size = data.len();
        self.objects.push(ObjectEntry {
            id,
            object_type,
            size,
            data,
        });
        self.header.count = self.objects.len();
        Ok(())
    }

    #[allow(dead_code)]
    /// Serialize pack to binary format
    pub fn serialize(&self) -> Result<Vec<u8>> {
        let mut buf = Vec::new();

        // Write header
        buf.extend_from_slice(b"CRUSTPACK\n");
        buf.extend_from_slice(format!("version: {}\n", self.header.version).as_bytes());
        buf.extend_from_slice(format!("count: {}\n", self.header.count).as_bytes());
        buf.extend_from_slice(b"\n");

        // Write objects
        for obj in &self.objects {
            buf.extend_from_slice(format!("id: {}\n", obj.id).as_bytes());
            buf.extend_from_slice(format!("type: {}\n", obj.object_type).as_bytes());
            buf.extend_from_slice(format!("size: {}\n", obj.size).as_bytes());
            buf.extend_from_slice(&obj.data);
        }

        // Write trailer (SHA256 of all preceding bytes)
        let mut hasher = Sha256::new();
        hasher.update(&buf);
        let digest = hasher.finalize();
        buf.extend_from_slice(&digest[..]);

        Ok(buf)
    }
}

/// CRUSTPACK reader - parses pack format
#[allow(dead_code)]
pub struct PackReader {
    header: PackHeader,
    objects: Vec<ObjectEntry>,
}

impl PackReader {
    /// Deserialize pack from binary format
    #[allow(dead_code)]
    pub fn deserialize(data: &[u8]) -> Result<Self> {
        if data.len() < 32 {
            return Err(anyhow!("PACK_MALFORMED: Pack data too short"));
        }

        // Extract trailer (last 32 bytes)
        let trailer = &data[data.len() - 32..];
        let pack_content = &data[..data.len() - 32];

        // Verify trailer
        let mut hasher = Sha256::new();
        hasher.update(pack_content);
        let expected_digest = hasher.finalize();

        if &expected_digest[..] != trailer {
            return Err(anyhow!("PACK_CHECKSUM_MISMATCH: Trailer SHA256 mismatch"));
        }

        // Parse header and objects
        let mut reader = pack_content;
        let header = Self::parse_header(&mut reader)?;

        let mut objects = Vec::new();
        for _ in 0..header.count {
            let obj = Self::parse_object(&mut reader)?;
            objects.push(obj);
        }

        Ok(PackReader { header, objects })
    }

    #[allow(dead_code)]
    fn parse_header(data: &mut &[u8]) -> Result<PackHeader> {
        // Read magic
        if !data.starts_with(b"CRUSTPACK\n") {
            return Err(anyhow!("PACK_MALFORMED: Invalid magic header"));
        }
        *data = &data[10..];

        // Read version
        let (version, remaining) = Self::read_line(data)?;
        if !version.starts_with("version: ") {
            return Err(anyhow!("PACK_MALFORMED: Missing version field"));
        }
        let version_str = &version[9..];
        let version: u32 = version_str
            .parse()
            .map_err(|_| anyhow!("PACK_MALFORMED: Invalid version"))?;
        *data = remaining;

        // Read count
        let (count_line, remaining) = Self::read_line(data)?;
        if !count_line.starts_with("count: ") {
            return Err(anyhow!("PACK_MALFORMED: Missing count field"));
        }
        let count_str = &count_line[7..];
        let count: usize = count_str
            .parse()
            .map_err(|_| anyhow!("PACK_MALFORMED: Invalid count"))?;
        *data = remaining;

        // Skip blank line
        let (blank, remaining) = Self::read_line(data)?;
        if !blank.is_empty() {
            return Err(anyhow!("PACK_MALFORMED: Expected blank line after header"));
        }
        *data = remaining;

        Ok(PackHeader { version, count })
    }

    #[allow(dead_code)]
    fn parse_object(data: &mut &[u8]) -> Result<ObjectEntry> {
        let (id_line, remaining) = Self::read_line(data)?;
        if !id_line.starts_with("id: ") {
            return Err(anyhow!("PACK_MALFORMED: Missing object id field"));
        }
        let id = id_line[4..].to_string();
        *data = remaining;

        let (type_line, remaining) = Self::read_line(data)?;
        if !type_line.starts_with("type: ") {
            return Err(anyhow!("PACK_MALFORMED: Missing object type field"));
        }
        let object_type = type_line[6..].to_string();
        *data = remaining;

        let (size_line, remaining) = Self::read_line(data)?;
        if !size_line.starts_with("size: ") {
            return Err(anyhow!("PACK_MALFORMED: Missing object size field"));
        }
        let size_str = &size_line[6..];
        let size: usize = size_str
            .parse()
            .map_err(|_| anyhow!("PACK_MALFORMED: Invalid size"))?;
        *data = remaining;

        // Read raw object data
        if data.len() < size {
            return Err(anyhow!("PACK_MALFORMED: Incomplete object data"));
        }
        let obj_data = &data[..size];
        let obj_vec = obj_data.to_vec();
        *data = &data[size..];

        Ok(ObjectEntry {
            id,
            object_type,
            size,
            data: obj_vec,
        })
    }

    #[allow(dead_code)]
    fn read_line(data: &[u8]) -> Result<(String, &[u8])> {
        if let Some(pos) = data.iter().position(|&b| b == b'\n') {
            let line = String::from_utf8_lossy(&data[..pos]).to_string();
            Ok((line, &data[pos + 1..]))
        } else {
            Err(anyhow!("PACK_MALFORMED: Line not terminated"))
        }
    }

    #[allow(dead_code)]
    pub fn get_header(&self) -> &PackHeader {
        &self.header
    }

    #[allow(dead_code)]
    pub fn get_objects(&self) -> &[ObjectEntry] {
        &self.objects
    }

    #[allow(dead_code)]
    pub fn into_objects(self) -> Vec<ObjectEntry> {
        self.objects
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pack_roundtrip() -> Result<()> {
        let mut writer = PackWriter::new();
        writer.add_object(
            "abc1234567890abcdef".to_string(),
            "blob".to_string(),
            b"hello world".to_vec(),
        )?;

        let packed = writer.serialize()?;
        let reader = PackReader::deserialize(&packed)?;

        assert_eq!(reader.get_header().count, 1);
        assert_eq!(reader.get_objects()[0].id, "abc1234567890abcdef");
        assert_eq!(reader.get_objects()[0].object_type, "blob");
        assert_eq!(reader.get_objects()[0].data, b"hello world");

        Ok(())
    }

    #[test]
    fn test_pack_multiple_objects() -> Result<()> {
        let mut writer = PackWriter::new();
        writer.add_object("aaa".to_string(), "blob".to_string(), b"data1".to_vec())?;
        writer.add_object("bbb".to_string(), "tree".to_string(), b"data2".to_vec())?;

        let packed = writer.serialize()?;
        let reader = PackReader::deserialize(&packed)?;

        assert_eq!(reader.get_header().count, 2);
        assert_eq!(reader.get_objects().len(), 2);

        Ok(())
    }
}
