use tss_esapi::structures::{Private, Public};
use tss_esapi::traits::{Marshall, UnMarshall};

use super::KeyHandleError;

const KEY_HANDLE_MAGIC: &[u8; 8] = b"PK4TPML2";
const UNPROTECTED: u8 = 0;
const PASSWORD_PROTECTED: u8 = 1;
pub(super) const AUTH_SALT_LENGTH: usize = 16;

pub(super) struct KeyHandleData {
    pub(super) private: Private,
    pub(super) public: Public,
    pub(super) auth_salt: Option<[u8; AUTH_SALT_LENGTH]>,
}

impl KeyHandleData {
    pub(super) fn serialize(&self) -> Result<Vec<u8>, tss_esapi::Error> {
        let private_bytes: &[u8] = self.private.as_slice();
        let public_bytes = self.public.marshall()?;
        let private_len = (private_bytes.len() as u32).to_be_bytes();
        let public_len = (public_bytes.len() as u32).to_be_bytes();
        let mut buf = Vec::with_capacity(
            8 + 1 + AUTH_SALT_LENGTH + 4 + private_bytes.len() + 4 + public_bytes.len(),
        );
        buf.extend_from_slice(KEY_HANDLE_MAGIC);
        match self.auth_salt {
            Some(salt) => {
                buf.push(PASSWORD_PROTECTED);
                buf.extend_from_slice(&salt);
            }
            None => buf.push(UNPROTECTED),
        }
        buf.extend_from_slice(&private_len);
        buf.extend_from_slice(private_bytes);
        buf.extend_from_slice(&public_len);
        buf.extend_from_slice(&public_bytes);
        Ok(buf)
    }

    pub(super) fn deserialize(data: &[u8]) -> Result<Self, KeyHandleError> {
        if data.len() < KEY_HANDLE_MAGIC.len() {
            return Err(KeyHandleError::UnrecognizedFormat);
        }
        if &data[..KEY_HANDLE_MAGIC.len()] != KEY_HANDLE_MAGIC {
            return Err(KeyHandleError::UnrecognizedFormat);
        }
        let body = &data[KEY_HANDLE_MAGIC.len()..];
        let Some((&local_authentication, body)) = body.split_first() else {
            return Err(KeyHandleError::TooShort {
                expected: 1,
                actual: 0,
            });
        };
        match local_authentication {
            UNPROTECTED => Self::deserialize_body(body, None),
            PASSWORD_PROTECTED if body.len() >= AUTH_SALT_LENGTH => {
                let salt = body[..AUTH_SALT_LENGTH].try_into().unwrap();
                Self::deserialize_body(&body[AUTH_SALT_LENGTH..], Some(salt))
            }
            PASSWORD_PROTECTED => Err(KeyHandleError::TooShort {
                expected: AUTH_SALT_LENGTH,
                actual: body.len(),
            }),
            value => Err(KeyHandleError::UnsupportedProtection(value)),
        }
    }

    fn deserialize_body(
        data: &[u8],
        auth_salt: Option<[u8; AUTH_SALT_LENGTH]>,
    ) -> Result<Self, KeyHandleError> {
        if data.len() < 8 {
            return Err(KeyHandleError::TooShort {
                expected: 8,
                actual: data.len(),
            });
        }
        let private_len = u32::from_be_bytes(data[0..4].try_into().unwrap()) as usize;
        let needed = 4 + private_len + 4;
        if data.len() < needed {
            return Err(KeyHandleError::TooShort {
                expected: needed,
                actual: data.len(),
            });
        }
        let private_bytes = &data[4..4 + private_len];
        let rest = &data[4 + private_len..];
        let public_len = u32::from_be_bytes(rest[0..4].try_into().unwrap()) as usize;
        if rest.len() < 4 + public_len {
            return Err(KeyHandleError::TooShort {
                expected: 4 + public_len,
                actual: rest.len(),
            });
        }
        let private =
            Private::try_from(private_bytes.to_vec()).map_err(KeyHandleError::InvalidPrivateKey)?;
        let public = Public::unmarshall(&rest[4..4 + public_len])
            .map_err(KeyHandleError::InvalidDevicePublicKey)?;
        Ok(KeyHandleData {
            private,
            public,
            auth_salt,
        })
    }
}
