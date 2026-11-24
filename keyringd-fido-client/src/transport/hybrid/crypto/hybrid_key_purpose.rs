/// An identifier that specifies what keys to derive from the common secret of HYBRID.
/// Separate the use of ad collation, tunnel identification, pre-shared key, and derive different values from the same secret.
#[derive(Clone, Copy)]
pub(crate) enum HybridKeyPurpose {
    Eid = 1,
    TunnelId = 2,
    Psk = 3,
}

#[cfg(test)]
mod tests {
    use super::*;

    keyringd_testkit::scenario! {
        group "HybridKeyPurpose" {
            case "Uses the key-purpose identifiers assigned by the hybrid transport" {
                why "These fixed HKDF domain-separation labels come from the Hybrid reference algorithm and must remain wire-compatible even though the numeric constants are not a standalone normative requirement."
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#hybrid-state-assisted"
                given "the three domain-separated derivations used by QR-initiated transactions" {
                    let purposes = [
                        HybridKeyPurpose::Eid,
                        HybridKeyPurpose::TunnelId,
                        HybridKeyPurpose::Psk,
                    ];
                }
                when "the EID, tunnel ID, and PSK purposes are encoded for HKDF info" {
                    let values = purposes.map(|purpose| purpose as u8);
                }
                then "their numeric identifiers are one, two, and three respectively" {
                    assert_eq!(values, [1, 2, 3]);
                }
            }
        }
    }
}
