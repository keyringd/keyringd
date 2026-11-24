#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct CtapHidCapabilities(u8);

impl CtapHidCapabilities {
    pub const WINK: Self = Self(0x01);
    pub const CBOR: Self = Self(0x04);
    pub const NO_MSG: Self = Self(0x08);
    pub const fn from_bits_retain(bits: u8) -> Self {
        Self(bits)
    }
    pub const fn bits(self) -> u8 {
        self.0
    }
    pub const fn contains(self, capability: Self) -> bool {
        self.0 & capability.0 == capability.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    keyringd_testkit::scenario! {
        group "CtapHidCapabilities::from_bits_retain" {
            case "Retains unknown CTAPHID capability bits" {
                why "Capability bytes are stored losslessly so future CTAPHID bits survive decode and re-encode without being mistaken for currently assigned behavior."
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#usb-hid-init"
                given "a capability byte containing assigned and future bits" {
                    let raw = 0x85;
                }
                when "the capability byte is represented by the protocol type" {
                    let capabilities = CtapHidCapabilities::from_bits_retain(raw);
                }
                then "all bits are retained while assigned capabilities remain queryable" {
                    assert_eq!(capabilities.bits(), raw);
                    assert!(capabilities.contains(CtapHidCapabilities::WINK));
                    assert!(capabilities.contains(CtapHidCapabilities::CBOR));
                    assert!(!capabilities.contains(CtapHidCapabilities::NO_MSG));
                }
            }
        }
    }
}
