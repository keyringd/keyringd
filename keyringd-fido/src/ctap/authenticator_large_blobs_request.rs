use crate::{
    ProtocolError,
    codec::{CborCodec, CborItem as Value},
};

/// CTAP authenticatorLargeBlobs request. Read and write fields cannot overlap.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AuthenticatorLargeBlobsRequest {
    Read {
        offset: u64,
        length: u64,
    },
    Write {
        offset: u64,
        data: Vec<u8>,
        total_length: Option<u64>,
        pin_uv_auth: Option<(Vec<u8>, u64)>,
    },
}
impl AuthenticatorLargeBlobsRequest {
    pub fn to_cbor_bytes(&self) -> Result<Vec<u8>, ProtocolError> {
        let mut fields = match self {
            Self::Read { offset, length } => vec![
                (1u64.into(), (*length).into()),
                (3u64.into(), (*offset).into()),
            ],
            Self::Write {
                offset,
                data,
                total_length,
                pin_uv_auth,
            } => {
                if (*offset == 0) != total_length.is_some() {
                    return Err(ProtocolError::UnexpectedType {
                        field: "largeBlobs.length",
                        expected: "length on first fragment only",
                    });
                }
                let mut fields = vec![
                    (2u64.into(), Value::ByteString(data.clone())),
                    (3u64.into(), (*offset).into()),
                ];
                if let Some(length) = total_length {
                    fields.push((4u64.into(), (*length).into()));
                }
                if let Some((auth, protocol)) = pin_uv_auth {
                    fields.push((5u64.into(), Value::ByteString(auth.clone())));
                    fields.push((6u64.into(), (*protocol).into()));
                }
                fields
            }
        };
        fields.sort_by_key(|(k, _)| if let Value::Unsigned(v) = k { *v } else { 0 });
        Ok(CborCodec::encode(Value::Map(fields))?)
    }
    pub fn from_cbor_bytes(bytes: &[u8]) -> Result<Self, ProtocolError> {
        use crate::codec::CborValue;
        let mut fields = CborValue::ctap_map(CborCodec::decode(bytes)?)?;
        let offset = CborValue::unsigned(
            fields
                .remove(&3)
                .ok_or(ProtocolError::MissingCtapField(3))?,
            "largeBlobs.offset",
        )?;
        let get = fields.remove(&1);
        let set = fields.remove(&2);
        let result = match (get, set) {
            (Some(get), None)
                if !fields.contains_key(&4)
                    && !fields.contains_key(&5)
                    && !fields.contains_key(&6) =>
            {
                Self::Read {
                    offset,
                    length: CborValue::unsigned(get, "largeBlobs.get")?,
                }
            }
            (None, Some(set)) => {
                let total_length = fields
                    .remove(&4)
                    .map(|v| CborValue::unsigned(v, "largeBlobs.length"))
                    .transpose()?;
                let auth = fields.remove(&5);
                let protocol = fields.remove(&6);
                let pin_uv_auth = match (auth, protocol) {
                    (None, None) => None,
                    (Some(auth), Some(protocol)) => Some((
                        CborValue::bytes(auth, "largeBlobs.pinUvAuthParam")?,
                        CborValue::unsigned(protocol, "largeBlobs.pinUvAuthProtocol")?,
                    )),
                    _ => return Err(Self::invalid()),
                };
                Self::Write {
                    offset,
                    data: CborValue::bytes(set, "largeBlobs.set")?,
                    total_length,
                    pin_uv_auth,
                }
            }
            _ => return Err(Self::invalid()),
        };
        result.to_cbor_bytes()?;
        Ok(result)
    }
    fn invalid() -> ProtocolError {
        ProtocolError::UnexpectedType {
            field: "largeBlobs",
            expected: "either a read or a write request",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wire_fields_are_exclusive_and_first_write_carries_total_length() {
        let read = AuthenticatorLargeBlobsRequest::Read {
            offset: 0,
            length: 960,
        };
        assert_eq!(
            read.to_cbor_bytes().unwrap(),
            [0xa2, 1, 0x19, 3, 0xc0, 3, 0]
        );
        assert_eq!(
            AuthenticatorLargeBlobsRequest::from_cbor_bytes(&read.to_cbor_bytes().unwrap())
                .unwrap(),
            read
        );
        for bytes in [
            &[0xa3, 1, 1, 2, 0x40, 3, 0][..],
            &[0xa3, 1, 1, 3, 0, 4, 1][..],
            &[0xa2, 2, 0x40, 3, 0][..],
        ] {
            assert!(AuthenticatorLargeBlobsRequest::from_cbor_bytes(bytes).is_err());
        }
        let write = AuthenticatorLargeBlobsRequest::Write {
            offset: 0,
            data: vec![0x80],
            total_length: Some(17),
            pin_uv_auth: Some((vec![7; 16], 1)),
        };
        assert_eq!(
            AuthenticatorLargeBlobsRequest::from_cbor_bytes(&write.to_cbor_bytes().unwrap())
                .unwrap(),
            write
        );
    }
}
