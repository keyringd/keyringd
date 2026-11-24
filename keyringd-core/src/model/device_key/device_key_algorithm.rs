#[derive(Debug, Clone, Copy)]
pub enum DeviceKeyAlgorithm {
    EcdsaP256,
    #[allow(dead_code)]
    Ed25519,
}
