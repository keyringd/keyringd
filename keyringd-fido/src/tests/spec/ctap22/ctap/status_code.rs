use crate::CtapStatusCode;

keyringd_testkit::scenario! {
    group "CtapStatusCode::from_byte" {
            case "Maps every assigned CTAP status code to its specified byte" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#error-responses"

                given "the complete set of status codes assigned by CTAP 2.2" {
                    let assigned = [
                        (0x00, CtapStatusCode::Ok),
                        (0x01, CtapStatusCode::InvalidCommand),
                        (0x02, CtapStatusCode::InvalidParameter),
                        (0x03, CtapStatusCode::InvalidLength),
                        (0x04, CtapStatusCode::InvalidSequence),
                        (0x05, CtapStatusCode::Timeout),
                        (0x06, CtapStatusCode::ChannelBusy),
                        (0x0a, CtapStatusCode::LockRequired),
                        (0x0b, CtapStatusCode::InvalidChannel),
                        (0x11, CtapStatusCode::CborUnexpectedType),
                        (0x12, CtapStatusCode::InvalidCbor),
                        (0x14, CtapStatusCode::MissingParameter),
                        (0x15, CtapStatusCode::LimitExceeded),
                        (0x16, CtapStatusCode::UnsupportedExtension),
                        (0x17, CtapStatusCode::FingerprintDatabaseFull),
                        (0x18, CtapStatusCode::LargeBlobStorageFull),
                        (0x19, CtapStatusCode::CredentialExcluded),
                        (0x21, CtapStatusCode::Processing),
                        (0x22, CtapStatusCode::InvalidCredential),
                        (0x23, CtapStatusCode::UserActionPending),
                        (0x24, CtapStatusCode::OperationPending),
                        (0x25, CtapStatusCode::NoOperations),
                        (0x26, CtapStatusCode::UnsupportedAlgorithm),
                        (0x27, CtapStatusCode::OperationDenied),
                        (0x28, CtapStatusCode::KeyStoreFull),
                        (0x2a, CtapStatusCode::NoOperationPending),
                        (0x2b, CtapStatusCode::UnsupportedOption),
                        (0x2c, CtapStatusCode::InvalidOption),
                        (0x2d, CtapStatusCode::KeepaliveCancel),
                        (0x2e, CtapStatusCode::NoCredentials),
                        (0x2f, CtapStatusCode::UserActionTimeout),
                        (0x30, CtapStatusCode::NotAllowed),
                        (0x31, CtapStatusCode::PinInvalid),
                        (0x32, CtapStatusCode::PinBlocked),
                        (0x33, CtapStatusCode::PinAuthInvalid),
                        (0x34, CtapStatusCode::PinAuthBlocked),
                        (0x35, CtapStatusCode::PinNotSet),
                        (0x36, CtapStatusCode::PinUvAuthTokenRequired),
                        (0x37, CtapStatusCode::PinPolicyViolation),
                        (0x38, CtapStatusCode::PinTokenExpired),
                        (0x39, CtapStatusCode::RequestTooLarge),
                        (0x3a, CtapStatusCode::ActionTimeout),
                        (0x3b, CtapStatusCode::UserPresenceRequired),
                        (0x3c, CtapStatusCode::UserVerificationBlocked),
                        (0x3d, CtapStatusCode::IntegrityFailure),
                        (0x3e, CtapStatusCode::InvalidSubcommand),
                        (0x3f, CtapStatusCode::UserVerificationInvalid),
                        (0x40, CtapStatusCode::UnauthorizedPermission),
                        (0x7f, CtapStatusCode::Other),
                    ];
                }
                when "each assigned byte is decoded and each status is encoded" {
                    let decoded = assigned.map(|(byte, _)| CtapStatusCode::from_byte(byte));
                    let encoded = assigned.map(|(_, status)| status.as_byte());
                }
                then "both directions use the exact assigned values" {
                    assert_eq!(decoded, assigned.map(|(_, status)| status));
                    assert_eq!(encoded, assigned.map(|(byte, _)| byte));
                }
            }
            case "Preserves an unassigned status code without treating it as success" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#error-responses"

                given "a byte that CTAP 2.2 does not assign to a status code" {
                    let byte = 0x13;
                }
                when "an unassigned status byte is decoded" {
                    let status = CtapStatusCode::from_byte(byte);
                }
                then "the original byte is retained and the status is not successful" {
                    assert_eq!(status, CtapStatusCode::Unknown(byte));
                    assert_eq!(status.as_byte(), byte);
                    assert!(!status.is_success());
                }
            }
    }
}
