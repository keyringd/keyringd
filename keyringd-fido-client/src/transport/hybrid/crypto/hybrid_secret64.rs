use zeroize::{Zeroize, ZeroizeOnDrop};

/// The 64-byte secret value obtained by derivation of the Hybrid key. Erases the row of bytes that are retained when discarded.
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct HybridSecret64(pub(crate) [u8; 64]);

impl HybridSecret64 {
    pub(crate) fn as_bytes(&self) -> &[u8; 64] {
        &self.0
    }
}
