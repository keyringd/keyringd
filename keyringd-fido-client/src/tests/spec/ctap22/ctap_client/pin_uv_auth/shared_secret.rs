use crate::ctap::pin_uv::PinUvAuthCryptoError;
use crate::ctap::pin_uv::PinUvAuthProtocol;
use crate::ctap::pin_uv::PinUvSharedSecret;
use crate::ctap::pin_uv::crypto as pin_uv_auth_crypto;
use crate::ctap::test_support::TestPinUvKeyAgreement;
use keyringd_fido::ctap::PinUvKeyAgreementKey;

keyringd_testkit::scenario! {
        case "Both protocols agree and encrypt common secrets" {
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#pinProto1"
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#pinProto2"

            given "Provide two PIN UV key exchanges for Protocol 1 and 2 each" {
                let protocols = [PinUvAuthProtocol::One, PinUvAuthProtocol::Two];
        }
        when "Derive common secrets and generate ciphertexts for each protocol" {
            let mut results = Vec::new();
            for protocol in protocols {

                let right = TestPinUvKeyAgreement::new(protocol);
                let (left_public, left_secret) = PinUvSharedSecret::establish(protocol, &right.public_key().unwrap()).unwrap();
                let (_, right_secret) = right.establish(&left_public).unwrap();
                let plaintext = vec![9; 32];
                let encrypted = left_secret.encrypt(&plaintext).unwrap();
                let decrypted = pin_uv_auth_crypto::decrypt(
                    protocol,
                    right_secret.expose_bytes(),
                    &encrypted,
                )
                .unwrap();
                results.push((
                    left_secret.expose_bytes().to_vec(),
                    right_secret.expose_bytes().to_vec(),
                    plaintext,
                    decrypted,
                ));
            }
        }
        then "Both protocols can agree to the same Secret and decrypt ciphertexts" {
            for (left_secret, right_secret, plaintext, decrypted) in results {
                assert_eq!(left_secret, right_secret);
                assert_eq!(decrypted, plaintext);
            }
        }

        }
    group "PinUvSharedSecret::establish" {
        case "Rejects a peer key whose coordinates are not a point on P-256" {
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#pinProto1"
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#pinProto2"

            given "a structurally valid PIN/UV key-agreement value whose all-zero coordinates are not a P-256 point" {
                let mut invalid_sec1_key = [0; 65];
                invalid_sec1_key[0] = 0x04;
                let invalid_peer_key = PinUvKeyAgreementKey::from_sec1_public_key(&invalid_sec1_key)
                    .expect("the wire model validates the uncompressed SEC1 shape only");

            }
            when "the client tries to establish the ECDH shared secret with the peer key" {
                let result = PinUvSharedSecret::establish(PinUvAuthProtocol::Two, &invalid_peer_key);
            }
            then "the P-256 implementation rejects the off-curve point before ECDH" {
                let error = result.as_ref().err().unwrap();
                assert!(std::error::Error::source(error).unwrap().is::<p256::elliptic_curve::Error>());
                assert!(matches!(
                    result,
                    Err(PinUvAuthCryptoError::InvalidPeerPublicKey(_))
                ));
            }
        }
    }
}
