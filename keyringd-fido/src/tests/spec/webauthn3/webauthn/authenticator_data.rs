use crate::webauthn::AuthenticatorData;
use crate::webauthn::FLAG_AT;
use crate::webauthn::FLAG_BE;
use crate::webauthn::FLAG_BS;
use crate::webauthn::FLAG_ED;
use crate::webauthn::FLAG_RESERVED_1;
use crate::webauthn::FLAG_RESERVED_2;
use crate::webauthn::FLAG_UP;
use crate::webauthn::FLAG_UV;
use crate::webauthn::ProtocolError;

keyringd_testkit::scenario! {
    group "AuthenticatorData::from_bytes" {
            case "Rejects authenticator data shorter than the 37-byte prefix" {
                spec "https://www.w3.org/TR/webauthn-3/#sctn-authenticator-data"

                verify "every truncated prefix length is rejected" {
                    for length in 0..37 {
                        assert!(matches!(AuthenticatorData::from_bytes(&vec![0; length]), Err(ProtocolError::AuthenticatorDataTooShort)));
                    }
                }
            }

            case "A round-trip conversion of Authenticator Data for Assertion" {
                spec "https://www.w3.org/TR/webauthn-3/#sctn-authenticator-data"

                given "Prepare Bytes" {
                    let mut bytes = vec![0x11; 32];
                    bytes.push(FLAG_UP | FLAG_UV);
                    bytes.extend_from_slice(&42u32.to_be_bytes());
                }
                when "Call AuthenticatorData::from_bytes" {
                    let decoded = AuthenticatorData::from_bytes(&bytes).unwrap();
                }
                then "A round-trip conversion of Authenticator Data for Assertion" {
                    assert!(decoded.user_present());
                    assert!(decoded.user_verified());
                    assert_eq!(decoded.sign_count(), 42);
                    assert_eq!(decoded.to_bytes().unwrap(), bytes);
                }

            }
            case "Attested ES256 key and extension can be transformed back and forth" {
                spec "https://www.w3.org/TR/webauthn-3/#sctn-authenticator-data"
                spec "https://www.w3.org/TR/webauthn-3/#sctn-attested-credential-data"

                given "Prepare Bytes" {
                    let mut bytes = vec![0x22; 32];
                    bytes.push(FLAG_UP | FLAG_AT | FLAG_ED);
                    bytes.extend_from_slice(&0u32.to_be_bytes());
                    bytes.extend_from_slice(&[0x33; 16]);
                    bytes.extend_from_slice(&3u16.to_be_bytes());
                    bytes.extend_from_slice(&[1, 2, 3]);
                    bytes.extend_from_slice(&[0xa5, 0x01, 0x02, 0x03, 0x26, 0x20, 0x01, 0x21, 0x58, 0x20]);
                    bytes.extend_from_slice(&[0x44; 32]);
                    bytes.extend_from_slice(&[0x22, 0x58, 0x20]);
                    bytes.extend_from_slice(&[0x55; 32]);
                    bytes.extend_from_slice(&[0xa1, 0x61, b'x', 0xf5]);
                }
                when "Call AuthenticatorData::from_bytes" {
                    let decoded = AuthenticatorData::from_bytes(&bytes).unwrap();
                    let attested = decoded.attested_credential_data().unwrap();
                }
                then "Attested ES256 key and extension can be transformed back and forth" {
                    assert_eq!(attested.aaguid(), &[0x33; 16]);
                    assert_eq!(attested.credential_id(), [1, 2, 3]);
                    assert_eq!(
                        decoded.extensions().unwrap().values().get("x").unwrap().as_bytes(),
                        &[0xf5]
                    );
                    assert_eq!(
                        attested
                            .credential_public_key()
                            .es256_sec1_public_key()
                            .unwrap()[0],
                        4
                    );
                    assert_eq!(decoded.to_bytes().unwrap(), bytes);
                }

            }
            case "Reject Reserved or Contradictory Backup Flags" {
                spec "https://www.w3.org/TR/webauthn-3/#sctn-authenticator-data"
                spec "https://www.w3.org/TR/webauthn-3/#sctn-credential-backup"

                given "Prepare Bytes" {
                    let mut bytes = vec![0; 37];
                }
                when "Set each Reserved Flag to Authenticator Data" {
                    let reserved_results = [FLAG_RESERVED_1, FLAG_RESERVED_2].map(|reserved| {
                        bytes[32] = reserved;
                        (reserved, AuthenticatorData::from_bytes(&bytes))
                    });
                }
                then "Verify that the results match ProtocolError::ReservedAuthenticatorDataFlags and ProtocolError::InvalidBackupFlags" {
                    for (reserved, result) in reserved_results {
                        assert!(matches!(result, Err(ProtocolError::ReservedAuthenticatorDataFlags(actual_flags)) if actual_flags == reserved));
                    }
                    bytes[32] = FLAG_BS;
                    assert!(matches!(AuthenticatorData::from_bytes(&bytes), Err(ProtocolError::InvalidBackupFlags)));
                }

            }

            case "Accepts every valid backup eligibility and state combination" {
                spec "https://www.w3.org/TR/webauthn-3/#table-backupStates"

                verify "single-device, eligible-not-backed-up, and eligible-backed-up flags decode" {
                    for flags in [0, FLAG_BE, FLAG_BE | FLAG_BS] {
                        let mut bytes = vec![0; 37];
                        bytes[32] = flags;
                        let decoded = AuthenticatorData::from_bytes(&bytes).unwrap();
                        assert_eq!(decoded.backup_eligible(), flags & FLAG_BE != 0);
                        assert_eq!(decoded.backup_state(), flags & FLAG_BS != 0);
                    }
                }
            }

            case "Rejects an attested credential ID longer than 1023 bytes" {
                spec "https://www.w3.org/TR/webauthn-3/#table-attestedCredentialData"

                given "authenticator data declaring 1023-byte and 1024-byte credential IDs followed by valid ES256 keys" {
                    fn data(credential_id_length: u16) -> Vec<u8> {
                        let mut bytes = vec![0x22; 32];
                        bytes.push(FLAG_AT);
                        bytes.extend_from_slice(&0u32.to_be_bytes());
                        bytes.extend_from_slice(&[0x33; 16]);
                        bytes.extend_from_slice(&credential_id_length.to_be_bytes());
                        bytes.extend_from_slice(&vec![0x44; usize::from(credential_id_length)]);
                        bytes.extend_from_slice(&[0xa5, 0x01, 0x02, 0x03, 0x26, 0x20, 0x01, 0x21, 0x58, 0x20]);
                        bytes.extend_from_slice(&[0x44; 32]);
                        bytes.extend_from_slice(&[0x22, 0x58, 0x20]);
                        bytes.extend_from_slice(&[0x55; 32]);
                        bytes
                    }

                    let maximum = data(1023);
                    let overlong = data(1024);
                }
                when "both attested credential data values are decoded" {
                    let maximum_result = AuthenticatorData::from_bytes(&maximum);
                    let overlong_result = AuthenticatorData::from_bytes(&overlong);
                }
                then "the maximum length is accepted and the next length is rejected" {
                    assert_eq!(
                        maximum_result
                            .unwrap()
                            .attested_credential_data()
                            .unwrap()
                            .credential_id()
                            .len(),
                        1023
                    );
                    assert!(matches!(
                        overlong_result,
                        Err(ProtocolError::CredentialIdTooLong { actual: 1024 })
                    ));
                }
            }
    }
}
