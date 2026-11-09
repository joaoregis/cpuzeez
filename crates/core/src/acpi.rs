//! ACPI table headers (S1-G04): the 36-byte header every table starts with.

#[derive(Debug, Clone, PartialEq)]
pub struct Header {
    pub signature: String,
    pub length: u32,
    pub revision: u8,
    pub oem_id: String,
    pub oem_table_id: String,
    pub oem_revision: u32,
    pub creator_id: String,
    pub creator_revision: u32,
}

fn text(b: &[u8]) -> String {
    String::from_utf8_lossy(b).trim_end_matches(['\0', ' ']).trim().to_string()
}

impl Header {
    pub fn parse(b: &[u8]) -> Option<Header> {
        let b = b.get(..36)?;
        let u32le = |o: usize| u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]]);
        Some(Header {
            signature: text(&b[0..4]),
            length: u32le(4),
            revision: b[8],
            oem_id: text(&b[10..16]),
            oem_table_id: text(&b[16..24]),
            oem_revision: u32le(24),
            creator_id: text(&b[28..32]),
            creator_revision: u32le(32),
        })
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn header() {
        let mut b = Vec::new();
        b.extend(b"FACP");
        b.extend(276u32.to_le_bytes());
        b.extend([6, 0x5A]);
        b.extend(b"ALASKA");
        b.extend(b"A M I \0\0");
        b.extend(0x0107_2009u32.to_le_bytes());
        b.extend(b"AMI ");
        b.extend(0x0001_0013u32.to_le_bytes());
        let h = super::Header::parse(&b).unwrap();
        assert_eq!((h.signature.as_str(), h.length, h.revision), ("FACP", 276, 6));
        assert_eq!((h.oem_id.as_str(), h.oem_table_id.as_str(), h.creator_id.as_str()), ("ALASKA", "A M I", "AMI"));
        assert!(super::Header::parse(&b[..35]).is_none());
    }
}
