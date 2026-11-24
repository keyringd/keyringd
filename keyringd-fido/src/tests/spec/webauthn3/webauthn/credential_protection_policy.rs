use crate::webauthn::*;

keyringd_testkit::scenario! {
    group "CredentialProtectionPolicy string conversion" {
            case "Maps every credential protection policy defined by CTAP" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#sctn-credProtect-extension"

                given "the three credentialProtectionPolicy strings accepted by the client extension" {
                    let values = [
                        "userVerificationOptional",
                        "userVerificationOptionalWithCredentialIDList",
                        "userVerificationRequired",
                    ];
                }
                when "the strings are converted to semantic policies" {
                    let policies = values.map(CredentialProtectionPolicy::from_string);
                }
                then "each policy maps back to its exact specification string" {
                    assert_eq!(
                        policies.map(|policy| policy.as_str().to_owned()),
                        values
                    );
                }
            }
    }
}
