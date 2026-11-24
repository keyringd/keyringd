use crate::{
    ProtocolError,
    codec::{CborCodec, CborItem as Value},
};
use sha2::{Digest, Sha256};

/// A CTAP large-blob array. Malformed entries are isolated on read and omitted
/// on write; valid maps retain unknown fields belonging to other clients.
#[derive(Default)]
pub struct LargeBlobArray {
    entries: Vec<Value>,
}
impl LargeBlobArray {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ProtocolError> {
        // CTAP requires treating an invalid checksum as the initial empty array.
        if bytes.len() < 17 {
            return Ok(Self::default());
        }
        let (data, checksum) = bytes.split_at(bytes.len() - 16);
        if Sha256::digest(data)[..16] != *checksum {
            return Ok(Self::default());
        }
        let Value::Array(entries) = CborCodec::decode(data)? else {
            return Err(Self::invalid());
        };
        Ok(Self { entries })
    }
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    pub fn entry(&self, index: usize) -> Option<super::LargeBlobMap<'_>> {
        super::LargeBlobMap::from_value(self.entries.get(index)?)
    }
    pub fn replace(
        &mut self,
        index: Option<usize>,
        ciphertext: Vec<u8>,
        nonce: [u8; 12],
        size: u64,
    ) {
        let entry = Value::Map(vec![
            (1u64.into(), Value::ByteString(ciphertext)),
            (2u64.into(), Value::ByteString(nonce.to_vec())),
            (3u64.into(), size.into()),
        ]);
        if let Some(index) = index {
            self.entries[index] = entry;
        } else {
            self.entries.push(entry);
        }
    }
    pub fn to_bytes(&self) -> Result<Vec<u8>, ProtocolError> {
        let mut bytes = CborCodec::encode(Value::Array(
            self.entries
                .iter()
                .filter(|entry| super::LargeBlobMap::from_value(entry).is_some())
                .cloned()
                .collect(),
        ))?;
        let hash = Sha256::digest(&bytes);
        bytes.extend_from_slice(&hash[..16]);
        Ok(bytes)
    }
    fn invalid() -> ProtocolError {
        ProtocolError::UnexpectedType {
            field: "large-blob array",
            expected: "an array of large-blob maps",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn encode(entries: Vec<Value>) -> Vec<u8> {
        let mut bytes = CborCodec::encode(Value::Array(entries)).unwrap();
        let checksum = Sha256::digest(&bytes);
        bytes.extend_from_slice(&checksum[..16]);
        bytes
    }
    #[test]
    fn malformed_entries_do_not_hide_valid_maps_and_are_omitted_when_writing() {
        let valid = Value::Map(vec![
            (1u64.into(), Value::ByteString(vec![7; 16])),
            (2u64.into(), Value::ByteString(vec![8; 12])),
            (3u64.into(), 9u64.into()),
            (
                4u64.into(),
                Value::TextString("another client's field".into()),
            ),
        ]);
        let incomplete = Value::Map(vec![(1u64.into(), Value::ByteString(vec![]))]);
        let bytes = encode(vec![0u64.into(), valid.clone(), incomplete]);
        let array = LargeBlobArray::from_bytes(&bytes).unwrap();
        assert!(array.entry(0).is_none());
        let map = array.entry(1).unwrap();
        assert_eq!(map.ciphertext, &[7; 16]);
        assert_eq!(map.nonce, &[8; 12]);
        assert_eq!(map.orig_size, 9);
        assert!(array.entry(2).is_none());
        assert_eq!(array.to_bytes().unwrap(), encode(vec![valid]));
    }
}
