#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CredentialProtectionPolicy {
    UserVerificationOptional,
    UserVerificationOptionalWithCredentialIdList,
    UserVerificationRequired,
    Unknown(String),
}

impl CredentialProtectionPolicy {
    pub fn from_string(value: impl Into<String>) -> Self {
        match value.into() {
            value if value == "userVerificationOptional" => Self::UserVerificationOptional,
            value if value == "userVerificationOptionalWithCredentialIDList" => {
                Self::UserVerificationOptionalWithCredentialIdList
            }
            value if value == "userVerificationRequired" => Self::UserVerificationRequired,
            value => Self::Unknown(value),
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::UserVerificationOptional => "userVerificationOptional",
            Self::UserVerificationOptionalWithCredentialIdList => {
                "userVerificationOptionalWithCredentialIDList"
            }
            Self::UserVerificationRequired => "userVerificationRequired",
            Self::Unknown(value) => value,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    keyringd_testkit::scenario! {
        group "CredentialProtectionPolicy::from_string" {
            case "Retains an unknown credential protection policy" {
                why "The value model preserves a future credProtect integer for lossless decoding while request validation separately limits values the client may send."
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#sctn-credProtect-extension"
                given "a credential protection policy introduced by a future specification" {
                    let value = "futurePolicy";
                }
                when "the string is converted to a semantic policy" {
                    let policy = CredentialProtectionPolicy::from_string(value);
                }
                then "the unknown string remains available for forward compatibility" {
                    assert_eq!(
                        policy,
                        CredentialProtectionPolicy::Unknown(value.to_owned())
                    );
                    assert_eq!(policy.as_str(), value);
                }
            }
        }
    }
}
