use crate::ctap::AuthenticatorGetInfoRequest;

keyringd_testkit::scenario! {
        case "GetInfo only accepts empty payloads" {
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#authenticatorGetInfo"

            verify "GetInfo only accepts empty payloads" {
                assert_eq!(
                    AuthenticatorGetInfoRequest::from_bytes(&[]).unwrap(),
                    AuthenticatorGetInfoRequest::new()
                );
                assert!(AuthenticatorGetInfoRequest::from_bytes(&[0xa0]).is_err());
                assert!(AuthenticatorGetInfoRequest::new().to_bytes().is_empty());
                }

        }
}
