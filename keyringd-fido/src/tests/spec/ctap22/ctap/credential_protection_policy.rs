use crate::CredentialProtectionPolicy;

keyringd_testkit::scenario! {
        case "Maps credential protection policies to their CTAP extension values" {
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#sctn-credProtect-extension"

            given "the three credential protection policies defined by CTAP" {
                let policies = [
                    CredentialProtectionPolicy::UserVerificationOptional,
                    CredentialProtectionPolicy::UserVerificationOptionalWithCredentialIdList,
                    CredentialProtectionPolicy::UserVerificationRequired,
                ];
            }
            when "their extension values are obtained" {
                let values = policies.map(CredentialProtectionPolicy::value);
            }
            then "the values are one, two, and three in increasing protection order" {
                assert_eq!(values, [1, 2, 3]);
            }
        }
}
