#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct PinUvAuthPermissions(u64);

impl PinUvAuthPermissions {
    pub const MAKE_CREDENTIAL: Self = Self(0x01);
    pub const GET_ASSERTION: Self = Self(0x02);
    pub const CREDENTIAL_MANAGEMENT: Self = Self(0x04);
    pub const BIO_ENROLLMENT: Self = Self(0x08);
    pub const LARGE_BLOB_WRITE: Self = Self(0x10);
    pub const AUTHENTICATOR_CONFIGURATION: Self = Self(0x20);
    pub const PERSISTENT_CREDENTIAL_MANAGEMENT_READ_ONLY: Self = Self(0x40);

    pub const fn from_bits_retain(bits: u64) -> Self {
        Self(bits)
    }

    pub const fn bits(self) -> u64 {
        self.0
    }

    pub const fn contains(self, permission: Self) -> bool {
        self.0 & permission.0 == permission.0
    }

    pub const fn union(self, permission: Self) -> Self {
        Self(self.0 | permission.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    keyringd_testkit::scenario! {
        group "PinUvAuthPermissions::union" {
            case "Combines permissions while retaining future bits" {
                why "Permission masks are stored losslessly so combining known permissions cannot erase future bits received from a newer protocol peer."
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#authenticatorClientPIN"
                given "a known permission and an unassigned future permission bit" {
                    let future = PinUvAuthPermissions::from_bits_retain(0x80);
                }
                when "the future bit is combined with getAssertion permission" {
                    let combined = PinUvAuthPermissions::GET_ASSERTION.union(future);
                }
                then "both the assigned and future permission bits remain intact" {
                    assert_eq!(combined.bits(), 0x82);
                    assert!(combined.contains(PinUvAuthPermissions::GET_ASSERTION));
                    assert!(combined.contains(future));
                }
            }
        }
    }
}
