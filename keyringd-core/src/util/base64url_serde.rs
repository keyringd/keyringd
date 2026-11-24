use serde::{Deserialize, Deserializer, Serializer};

fn encode(data: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(data)
}

fn decode(s: &str) -> Result<Vec<u8>, String> {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(s)
        .map_err(|error| error.to_string())
}

pub fn serialize<S>(data: &[u8], serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&encode(data))
}

pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    decode(&s).map_err(serde::de::Error::custom)
}

#[cfg(test)]
mod tests {
    use super::*;

    keyringd_testkit::scenario! {
        group "decode" {
            case "Round-trips binary data without padding through URL-safe Base64" {
                given "binary data whose standard Base64 representation would use reserved URL characters" {
                    let data = vec![0x04, 0x01, 0x02, 0x03, 0xFF, 0xFE];
                }
                when "the data is encoded without padding and decoded again" {
                    let encoded = encode(&data);
                    let decoded = decode(&encoded).unwrap();
                }
                then "the encoding contains no standard Base64 URL-reserved characters or padding and restores the original bytes" {
                    assert!(!encoded.contains('+'));
                    assert!(!encoded.contains('/'));
                    assert!(!encoded.contains('='));
                    assert_eq!(decoded, data);
                }
            }
        }
        group "encode" {
            case "Represents an empty byte sequence as an empty Base64url string" {
                given "an empty byte sequence" {
                    let data: Vec<u8> = Vec::new();
                }
                when "the empty sequence is encoded and the resulting empty string is decoded" {
                    let encoded = encode(&data);
                    let decoded = decode(&encoded).unwrap();
                }
                then "both encoded and decoded representations remain empty" {
                    assert_eq!(encoded, "");
                    assert!(decoded.is_empty());
                }
            }
        }
    }
}
