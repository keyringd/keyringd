use crate::PinUvAuthCryptoError;
use crate::PinUvAuthProtocol;
use crate::ctap::pin_uv::authenticate;
use crate::ctap::pin_uv::decrypt;
use crate::ctap::pin_uv::derive_secret;
use crate::ctap::pin_uv::encrypt;
use aes::cipher::BlockModeDecrypt;
use aes::cipher::KeyIvInit;
use aes::cipher::block_padding::NoPadding;

keyringd_testkit::scenario! {
    group "PIN/UV auth protocol cryptography" {
            case "Protocol One derives the shared secret with SHA-256" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#pinProto1"

                given "a fixed 32-byte ECDH output" {
                    let ecdh_secret = hex_literal::hex!(
                        "000102030405060708090a0b0c0d0e0f
                         101112131415161718191a1b1c1d1e1f"
                    );
                }
                when "Protocol One derives its shared secret" {
                    let secret = derive_secret(PinUvAuthProtocol::One, &ecdh_secret).unwrap();
                }
                then "the result equals SHA-256 of the ECDH output" {
                    assert_eq!(
                        secret,
                        hex_literal::hex!(
                            "630dcd2966c4336691125448bbb25b4f
                             f412a49c732db2c8abc1b8581bd710dd"
                        )
                    );
                }
            }
            case "Protocol Two derives independent HMAC and AES keys" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#pinProto2"

                given "a fixed 32-byte ECDH output" {
                    let ecdh_secret = hex_literal::hex!(
                        "000102030405060708090a0b0c0d0e0f
                         101112131415161718191a1b1c1d1e1f"
                    );
                }
                when "Protocol Two performs the two specified HKDF invocations" {
                    let secret = derive_secret(PinUvAuthProtocol::Two, &ecdh_secret).unwrap();
                }
                then "the HMAC key precedes the independently derived AES key" {
                    assert_eq!(
                        secret,
                        hex_literal::hex!(
                            "a689b3b92a6ebab91192408da9c4f05c
                     674a2bc5f938d613077716c719a8df39
                     0f6ff2ef211829c11638ef2893ea02ed
                     f195658c0572393e7680d93bc2b58d44"
                        )
                    );
                }
            }
            case "Protocol One uses a zero IV and a truncated authenticator" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#pinProto1"

                given "a fixed shared secret, plaintext, and message" {
                    let secret = hex_literal::hex!(
                        "630dcd2966c4336691125448bbb25b4f
                         f412a49c732db2c8abc1b8581bd710dd"
                    );
                    let plaintext = hex_literal::hex!(
                        "000102030405060708090a0b0c0d0e0f
                         101112131415161718191a1b1c1d1e1f"
                    );
                    let message = b"CTAP2 test message";
                }
                when "Protocol One encrypts and authenticates the inputs" {
                    let ciphertext = encrypt(PinUvAuthProtocol::One, &secret, &plaintext).unwrap();
                    let signature = authenticate(PinUvAuthProtocol::One, &secret, message).unwrap();
                }
                then "AES-256-CBC uses the all-zero IV and HMAC-SHA-256 is truncated to 16 bytes" {
                    assert_eq!(
                        ciphertext,
                        hex_literal::hex!(
                            "5b1b8c62089fae8afdcde68081977f19
                             e6a0b8d59b6818113cd771f0867e8903"
                        )
                    );
                    assert_eq!(signature, hex_literal::hex!("327724499da4aef2be68b012524bf081"));
                }
            }
            case "Rejects PIN UV plaintext that is not aligned to an AES block" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#authnrClientPin-puaprot-abstract-dfn"
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#pinProto1"
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#pinProto2"

                given "Protocol One and Two secrets and plaintexts immediately around one AES block" {
                    let cases = [
                        (PinUvAuthProtocol::One, vec![0x11; 32]),
                        (PinUvAuthProtocol::Two, vec![0x22; 64]),
                    ];
                    let invalid_lengths = [15, 17];
                }
                when "each protocol encrypts each non-aligned plaintext" {
                    let results = cases.iter().flat_map(|(protocol, secret)| {
                        invalid_lengths.map(|length| {
                            (*protocol, encrypt(*protocol, secret, &vec![0x33; length]))
                        })
                    }).collect::<Vec<_>>();
                }
                then "both protocols reject lengths below and above the 16-byte boundary" {
                    for (protocol, result) in results {
                        assert!(matches!(
                            result,
                            Err(PinUvAuthCryptoError::InvalidLength {
                                protocol: actual,
                            })
                            if actual == protocol.version()
                        ));
                    }
                }
            }
            case "Protocol One decrypts complete AES blocks and rejects a partial block" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#pinProto1"

                given "a fixed Protocol One secret, a known two-block ciphertext, and a partial ciphertext" {
                    let secret = hex_literal::hex!(
                        "630dcd2966c4336691125448bbb25b4f
                         f412a49c732db2c8abc1b8581bd710dd"
                    );
                    let ciphertext = hex_literal::hex!(
                        "5b1b8c62089fae8afdcde68081977f19
                         e6a0b8d59b6818113cd771f0867e8903"
                    );
                    let partial_ciphertext = [0x5b; 15];
                }
                when "Protocol One decrypts both ciphertexts" {
                    let plaintext = decrypt(PinUvAuthProtocol::One, &secret, &ciphertext).unwrap();
                    let partial_result =
                        decrypt(PinUvAuthProtocol::One, &secret, &partial_ciphertext);
                }
                then "the complete blocks use the all-zero IV and the partial block is rejected" {
                    assert_eq!(
                        plaintext,
                        hex_literal::hex!(
                            "000102030405060708090a0b0c0d0e0f
                             101112131415161718191a1b1c1d1e1f"
                        )
                    );
                    assert!(matches!(
                        partial_result,
                        Err(PinUvAuthCryptoError::InvalidLength { protocol: 1 })
                    ));
                }
            }
            case "Protocol Two reads the IV prefix and uses a full authenticator" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#pinProto2"

                given "a fixed Protocol Two secret, IV-prefixed ciphertext, and message" {
                    let secret = hex_literal::hex!(
                        "a689b3b92a6ebab91192408da9c4f05c
                 674a2bc5f938d613077716c719a8df39
                 0f6ff2ef211829c11638ef2893ea02ed
                 f195658c0572393e7680d93bc2b58d44"
                    );
                    let ciphertext = hex_literal::hex!(
                        "f0f1f2f3f4f5f6f7f8f9fafbfcfdfeff
                 02d74b06ea1a73bebab0c7575dc4ee31
                 66492bc29d89a9efcc089aabeaf40e15"
                    );
                    let message = b"CTAP2 test message";
                }
                when "Protocol Two decrypts and authenticates the inputs" {
                    let plaintext = decrypt(PinUvAuthProtocol::Two, &secret, &ciphertext).unwrap();
                    let signature = authenticate(PinUvAuthProtocol::Two, &secret, message).unwrap();
                }
                then "the first 16 bytes are treated as the IV and HMAC-SHA-256 remains 32 bytes" {
                    assert_eq!(
                        plaintext,
                        hex_literal::hex!(
                            "000102030405060708090a0b0c0d0e0f
                             101112131415161718191a1b1c1d1e1f"
                        )
                    );
                    assert_eq!(
                        signature,
                        hex_literal::hex!(
                            "eb25ccf68940c2ed61e9a505cb113df8
                             8f940dae976bb2e5709441fdd0e5daba"
                        )
                    );
                }
            }
            case "Protocol Two prefixes a fresh IV and encrypts with the AES half of the secret" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#pinProto2"

                given "a 64-byte Protocol Two secret and a two-block plaintext" {
                    let secret = [0x44; 64];
                    let plaintext = [0x2a; 32];
                }
                when "the plaintext is encrypted twice" {
                    let first = encrypt(PinUvAuthProtocol::Two, &secret, &plaintext).unwrap();
                    let second = encrypt(PinUvAuthProtocol::Two, &secret, &plaintext).unwrap();
                    let independently_decrypted = cbc::Decryptor::<aes::Aes256>::new_from_slices(
                        &secret[32..],
                        &first[..16],
                    )
                    .unwrap()
                    .decrypt_padded_vec::<NoPadding>(&first[16..])
                    .unwrap();
                }
                then "each result is IV followed by unpadded ciphertext under the AES key" {
                    assert_eq!(first.len(), 16 + plaintext.len());
                    assert_eq!(second.len(), 16 + plaintext.len());
                    assert_ne!(&first[..16], &second[..16]);
                    assert_eq!(independently_decrypted, plaintext);
                }
            }
            case "Protocol Two rejects a ciphertext shorter than its IV" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#pinProto2"

                given "a valid Protocol Two secret and a 15-byte ciphertext" {
                    let secret = [0x44; 64];
                    let ciphertext = [0x55; 15];
                }
                when "Protocol Two decrypts the ciphertext" {
                    let result = decrypt(PinUvAuthProtocol::Two, &secret, &ciphertext);
                }
                then "the missing 16-byte IV is rejected" {
                    assert!(matches!(
                        result,
                        Err(PinUvAuthCryptoError::InvalidLength { protocol: 2 })
                    ));
                }
            }
            case "Protocol Two accepts an IV-only ciphertext as an empty plaintext" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#pinProto2"

                given "a valid Protocol Two secret and a ciphertext containing only its 16-byte IV" {
                    let secret = [0x44; 64];
                    let ciphertext = [0x55; 16];
                }
                when "Protocol Two decrypts the minimum-length ciphertext allowed by the specification" {
                    let plaintext = decrypt(PinUvAuthProtocol::Two, &secret, &ciphertext).unwrap();
                }
                then "the decrypted plaintext is empty" {
                    assert!(plaintext.is_empty());
                }
            }
    }
}
