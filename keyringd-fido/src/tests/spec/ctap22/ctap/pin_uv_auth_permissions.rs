use crate::ctap::PinUvAuthPermissions;

keyringd_testkit::scenario! {
        case "Maps PIN UV authorization permissions to their CTAP bits" {
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#authenticatorClientPIN"

            given "all PIN UV authorization permissions defined by CTAP 2.2" {
                let permissions = [
                    PinUvAuthPermissions::MAKE_CREDENTIAL,
                    PinUvAuthPermissions::GET_ASSERTION,
                    PinUvAuthPermissions::CREDENTIAL_MANAGEMENT,
                    PinUvAuthPermissions::BIO_ENROLLMENT,
                    PinUvAuthPermissions::LARGE_BLOB_WRITE,
                    PinUvAuthPermissions::AUTHENTICATOR_CONFIGURATION,
                    PinUvAuthPermissions::PERSISTENT_CREDENTIAL_MANAGEMENT_READ_ONLY,
                ];
            }
            when "their permission bits are obtained" {
                let bits = permissions.map(PinUvAuthPermissions::bits);
            }
            then "each permission occupies its assigned independent bit" {
                assert_eq!(bits, [0x01, 0x02, 0x04, 0x08, 0x10, 0x20, 0x40]);
            }
        }
}
