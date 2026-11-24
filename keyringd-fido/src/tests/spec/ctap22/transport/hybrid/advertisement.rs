use crate::transport::hybrid::HybridAdvertisement;

keyringd_testkit::scenario! {
        case "Splits a decrypted hybrid advertisement into its specified fields" {
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#hybrid-state-assisted"

            given "a 16-byte advertisement plaintext containing flags, nonce, routing ID, and domain" {
                let plaintext = [
                    0x20,
                    0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a,
                    0xa1, 0xa2, 0xa3,
                    0x34, 0x12,
                ];
            }
            when "the plaintext is interpreted as a hybrid advertisement" {
                let advertisement = HybridAdvertisement::new(plaintext);
            }
            then "the nonce and routing ID retain their wire order and the domain is little-endian" {
                assert_eq!(advertisement.plaintext(), &plaintext);
                assert_eq!(advertisement.nonce(), &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
                assert_eq!(advertisement.routing_id(), &[0xa1, 0xa2, 0xa3]);
                assert_eq!(advertisement.encoded_tunnel_server_domain(), 0x1234);
            }
        }
}
