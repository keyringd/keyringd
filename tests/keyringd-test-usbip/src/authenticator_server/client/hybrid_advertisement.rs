use super::AuthenticatorServerError;
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HybridAdvertisement {
    pub service_uuid: String,
    pub service_data_hex: String,
    pub tunnel_domain: String,
    pub tunnel_path: String,
}
impl HybridAdvertisement {
    pub fn service_data(&self) -> Result<[u8; 20], AuthenticatorServerError> {
        if self.service_uuid != "fff9"
            || self.service_data_hex.len() != 40
            || !self
                .service_data_hex
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(AuthenticatorServerError::Protocol(
                "invalid Hybrid advertisement".into(),
            ));
        }
        let mut data = [0; 20];
        for (index, byte) in data.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&self.service_data_hex[index * 2..index * 2 + 2], 16)
                .map_err(|_| {
                    AuthenticatorServerError::Protocol("invalid Hybrid service data".into())
                })?;
        }
        Ok(data)
    }
}
