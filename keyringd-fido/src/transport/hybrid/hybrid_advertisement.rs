/// Authenticated and decrypted Hybrid ads. Read connection Nonce, routing ID, tunnel server information.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HybridAdvertisement {
    plaintext: [u8; 16],
}

impl HybridAdvertisement {
    pub const fn new(plaintext: [u8; 16]) -> Self {
        Self { plaintext }
    }
    pub const fn plaintext(&self) -> &[u8; 16] {
        &self.plaintext
    }
    pub fn nonce(&self) -> &[u8; 10] {
        self.plaintext[1..11].try_into().expect("fixed range")
    }
    pub fn routing_id(&self) -> &[u8; 3] {
        self.plaintext[11..14].try_into().expect("fixed range")
    }
    pub fn tunnel_server_domain(&self) -> Option<String> {
        super::tunnel_server_domain::decode(self.encoded_tunnel_server_domain())
    }
    pub const fn encoded_tunnel_server_domain(&self) -> u16 {
        u16::from_le_bytes([self.plaintext[14], self.plaintext[15]])
    }
}
