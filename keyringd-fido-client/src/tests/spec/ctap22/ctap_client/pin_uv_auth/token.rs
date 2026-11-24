use crate::PinUvAuthProtocol;
use crate::ctap::pin_uv::PinUvAuthToken;

keyringd_testkit::scenario! {
    group "PinUvAuthToken::new" {
            case "Zeroizes the PIN UV auth token when its operation-scoped value is dropped" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#authenticatorClientPIN"

                verify "the token type implements both explicit and drop-time zeroization" {
                    fn assert_zeroizing<T: zeroize::Zeroize + zeroize::ZeroizeOnDrop>() {}
                    assert_zeroizing::<PinUvAuthToken>();
                }
            }
            case "Accepts both token lengths permitted by Protocol One" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#pinProto1"

                given "16-byte and 32-byte PIN/UV auth token values" {
                    let lengths = [16, 32];
                }
                when "the values are interpreted as Protocol One tokens" {
                    let results = lengths.map(|length| {
                        PinUvAuthToken::new(PinUvAuthProtocol::One, vec![0x11; length])
                    });
                }
                then "both token lengths are accepted" {
                    assert!(results.into_iter().all(|result| result.is_ok()));
                }
            }
            case "Rejects Protocol One tokens whose length is neither 16 nor 32 bytes" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#puap1keyagmnt-public-point"

                given "15-byte, 17-byte, 31-byte, and 33-byte PIN/UV auth token values" {
                    let lengths = [15, 17, 31, 33];
                }
                when "the values are interpreted as Protocol One tokens" {
                    let results = lengths.map(|length| {
                        PinUvAuthToken::new(PinUvAuthProtocol::One, vec![0x11; length])
                    });
                }
                then "every length outside the two permitted sizes is rejected" {
                    assert!(results.into_iter().all(|result| result.is_err()));
                }
            }
            case "Requires exactly 32 bytes for a Protocol Two token" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#pinProto2"

                given "31-byte, 32-byte, and 33-byte PIN/UV auth token values" {
                    let lengths = [31, 32, 33];
                }
                when "the values are interpreted as Protocol Two tokens" {
                    let results = lengths.map(|length| {
                        PinUvAuthToken::new(PinUvAuthProtocol::Two, vec![0x22; length])
                    });
                }
                then "only the 32-byte token is accepted" {
                    assert!(results[0].is_err());
                    assert!(results[1].is_ok());
                    assert!(results[2].is_err());
                }
            }
    }
}
