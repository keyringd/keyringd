use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, Deserialize, Serialize)]
#[serde(transparent)]
pub struct DevicePrivateKeyHandle {
    #[serde(with = "crate::util::base64url_serde")]
    bytes: Vec<u8>,
}

impl DevicePrivateKeyHandle {
    pub fn new(bytes: Vec<u8>) -> Self {
        Self { bytes }
    }
}

impl fmt::Debug for DevicePrivateKeyHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("DevicePrivateKeyHandle")
            .field(&"<redacted>")
            .finish()
    }
}

impl AsRef<[u8]> for DevicePrivateKeyHandle {
    fn as_ref(&self) -> &[u8] {
        &self.bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    keyringd_testkit::scenario! {
        group "DevicePrivateKeyHandle::fmt" {
            case "Redacts private handle bytes from diagnostics" {
                verify "debug output replaces private handle bytes with a redaction marker" {
                    let handle = DevicePrivateKeyHandle::new(b"private-handle".to_vec());
                    assert_eq!(
                        format!("{handle:?}"),
                        "DevicePrivateKeyHandle(\"<redacted>\")"
                    );
                }
            }
        }
        group "DevicePrivateKeyHandle serialization" {
            case "Preserves private handle bytes through JSON serialization" {
                verify "deserializing the serialized handle restores its original bytes" {
                    let handle = DevicePrivateKeyHandle::new(b"private-handle".to_vec());
                    let restored: DevicePrivateKeyHandle =
                        serde_json::from_value(serde_json::to_value(&handle).unwrap()).unwrap();
                    assert_eq!(restored.as_ref(), b"private-handle");
                }
            }
        }
    }
}
