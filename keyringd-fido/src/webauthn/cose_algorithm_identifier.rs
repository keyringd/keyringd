/// WebAuthn `COSEAlgorithmIdentifier`, defined as a WebIDL `long`.
///
/// This is intentionally open: values unknown to this library remain representable.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(transparent)]
pub struct CoseAlgorithmIdentifier(i32);

impl CoseAlgorithmIdentifier {
    pub const ES256: Self = Self(-7);

    pub const fn new(value: i32) -> Self {
        Self(value)
    }

    pub const fn value(self) -> i32 {
        self.0
    }
}

impl From<i32> for CoseAlgorithmIdentifier {
    fn from(value: i32) -> Self {
        Self(value)
    }
}

impl From<CoseAlgorithmIdentifier> for i32 {
    fn from(value: CoseAlgorithmIdentifier) -> Self {
        value.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    keyringd_testkit::scenario! {
        group "CoseAlgorithmIdentifier::new" {
            case "Preserves algorithm identifiers not known by this crate" {
                why "COSE algorithm registries evolve independently, so the protocol model must retain an unfamiliar integer without falsely claiming support for that algorithm."
                spec "https://www.w3.org/TR/webauthn-3/#typedefdef-cosealgorithmidentifier"
                verify "an arbitrary WebIDL long value remains representable" {
                    assert_eq!(CoseAlgorithmIdentifier::new(1234).value(), 1234);
                }
            }
        }
    }
}
