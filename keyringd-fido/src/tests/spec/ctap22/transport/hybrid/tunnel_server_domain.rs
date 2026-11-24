use crate::transport::hybrid::decode;

keyringd_testkit::scenario! {
        case "Decode assigned Domain and Hashed Domain" {
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#sctn-hybrid"

            verify "Decode assigned Domain and Hashed Domain" {
                assert_eq!(decode(0).as_deref(), Some("cable.ua5v.com"));
                assert_eq!(decode(1).as_deref(), Some("cable.auth.com"));
                assert_eq!(
                    decode(0x1234).as_deref(),
                    Some("cable.x6dnkxkmx2odd.org")
                );
                }

        }
}
