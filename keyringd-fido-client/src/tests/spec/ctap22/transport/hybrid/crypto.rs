use crate::transport::hybrid::crypto::HybridCryptoError;
use crate::transport::hybrid::crypto::HybridKeyPurpose;
use crate::transport::hybrid::crypto::HybridSecret64;
use crate::transport::hybrid::crypto::decrypt_advertisement;
use crate::transport::hybrid::crypto::derive;
use aes::Aes256;
use aes::Block;
use aes::cipher::{BlockCipherEncrypt, KeyInit};
use hmac::Hmac;
use hmac::Mac;
use sha2::Sha256;

keyringd_testkit::scenario! {
        case "Derives distinct tunnel identifiers and handshake PSKs with the assigned salts and purposes" {
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#hybrid-state-assisted"

            given "a fixed 16-byte QR secret and a fixed 16-byte decrypted advertisement used as the PSK salt" {
                let secret = hex_literal::hex!("000102030405060708090a0b0c0d0e0f");
                let advertisement = hex_literal::hex!("f0f1f2f3f4f5f6f7f8f9fafbfcfdfeff");
            }
            when "HKDF-SHA-256 derives tunnel-ID material without a salt and PSK material with the advertisement as salt" {
                let tunnel = derive(&secret, None, HybridKeyPurpose::TunnelId).unwrap();
                let psk = derive(&secret, Some(&advertisement), HybridKeyPurpose::Psk).unwrap();
            }
            then "both 64-byte outputs match independently calculated vectors and remain domain separated" {
                assert_eq!(
                    tunnel.as_bytes(),
                    &hex_literal::hex!(
                        "de1d8150b45f03349b2b00a52e2e893e
                 af11829784e31d8d9f3764ea46cdec21
                 6402f7b46a2acbb7e5c32bd288e82e57
                 f554373926083617fa87669cf90b1020"
                    )
                );
                assert_eq!(
                    psk.as_bytes(),
                    &hex_literal::hex!(
                        "c5ce300a243fe018598870482078793c
                 df544096ffb41322e19246fa69ff91f4
                 b5dab48b2a4f8d5a06e475d1254224f
                 b8b5b49992db89956be6e804980550f3a"
                    )
                );
                assert_ne!(tunnel.as_bytes(), psk.as_bytes());
            }
        }
        case "Match known EID derivation vector" {
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#hybrid-state-assisted"

            given "Prepare an EID Key/Nonce/Routing ID for a Vector known Vector" {
                let input = [
                    0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd,
                    0xee, 0xff,
                ];
        }
        when "Call Derive" {
            let actual = derive(&input, None, HybridKeyPurpose::Eid).unwrap();
            let expected = [
                0xef, 0xaf, 0xab, 0x5b, 0x2c, 0x84, 0xa1, 0x1c, 0x80, 0xe3, 0xad, 0x07, 0x70, 0x35,
                0x31, 0x38, 0xb4, 0x14, 0xa8, 0x59, 0xcc, 0xd3, 0xaf, 0xcc, 0x99, 0xe3, 0xd3, 0x25,
                0x0d, 0xba, 0x65, 0x08, 0x4e, 0xde, 0x8e, 0x38, 0xe7, 0x54, 0x32, 0x61, 0x7c, 0x0c,
                0xca, 0xe1, 0xff, 0xe5, 0xd8, 0x14, 0x3d, 0xf0, 0xdb, 0x0c, 0xd6, 0xd2, 0x96, 0xf4,
                0x89, 0x41, 0x9c, 0xd6, 0x41, 0x1e, 0xe5, 0x05,
            ];
        }
        then "Match known EID derivation vector" {
            assert_eq!(actual.as_bytes(), &expected);
        }

        }
        case "Authenticate and decrypt Advertisement" {
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#hybrid-state-assisted"

            given "Prepare EID Key and Plaintext to generate authenticated Advertisement" {
                let eid = HybridSecret64([0x44; 64]);
                let plaintext = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 0xaa, 0xbb, 0xcc, 1, 0];
                let cipher = Aes256::new_from_slice(&eid.as_bytes()[..32]).unwrap();
                let mut body = Block::try_from(plaintext.as_slice()).unwrap();
                cipher.encrypt_block(&mut body);
                let mut mac = <Hmac<Sha256> as KeyInit>::new_from_slice(&eid.as_bytes()[32..]).unwrap();
                mac.update(&body);
                let tag = mac.finalize().into_bytes();
                let mut advertisement = body.to_vec();
        }
        when "Grant the correct 4-byte authentication tag to the ciphertext" {
            advertisement.extend_from_slice(&tag[..4]);
        }
        then "Verify that the results match HybridCryptoError::AdvertisementAuthentication" {
            assert_eq!(
                decrypt_advertisement(&eid, &advertisement).unwrap(),
                plaintext
            );
            advertisement[19] ^= 1;
            assert!(matches!(
                decrypt_advertisement(&eid, &advertisement),
                Err(HybridCryptoError::AdvertisementAuthentication)
            ));
        }

        }
        case "Rejects advertisement service data that is not exactly 20 bytes" {
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#hybrid-state-assisted"

            given "an EID key and candidate service-data values shorter and longer than 20 bytes" {
                let eid = HybridSecret64([0x44; 64]);
                let short = [0; 19];
                let long = [0; 21];
            }
            when "the candidates are processed as hybrid advertisements" {
                let short_result = decrypt_advertisement(&eid, &short);
                let long_result = decrypt_advertisement(&eid, &long);
            }
            then "both lengths are rejected before authentication or decryption" {
                assert!(matches!(
                    short_result,
                    Err(HybridCryptoError::InvalidAdvertisementLength(19))
                ));
                assert!(matches!(
                    long_result,
                    Err(HybridCryptoError::InvalidAdvertisementLength(21))
                ));
            }

        }
}
