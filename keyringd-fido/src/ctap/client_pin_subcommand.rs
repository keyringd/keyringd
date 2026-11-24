#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ClientPinSubcommand {
    GetPinRetries,
    GetKeyAgreement,
    SetPin,
    ChangePin,
    GetPinToken,
    GetPinUvAuthTokenUsingUvWithPermissions,
    GetUvRetries,
    GetPinUvAuthTokenUsingPinWithPermissions,
    Unknown(u64),
}

impl ClientPinSubcommand {
    pub const fn from_u64(value: u64) -> Self {
        match value {
            0x01 => Self::GetPinRetries,
            0x02 => Self::GetKeyAgreement,
            0x03 => Self::SetPin,
            0x04 => Self::ChangePin,
            0x05 => Self::GetPinToken,
            0x06 => Self::GetPinUvAuthTokenUsingUvWithPermissions,
            0x07 => Self::GetUvRetries,
            0x09 => Self::GetPinUvAuthTokenUsingPinWithPermissions,
            value => Self::Unknown(value),
        }
    }

    pub const fn as_u64(self) -> u64 {
        match self {
            Self::GetPinRetries => 0x01,
            Self::GetKeyAgreement => 0x02,
            Self::SetPin => 0x03,
            Self::ChangePin => 0x04,
            Self::GetPinToken => 0x05,
            Self::GetPinUvAuthTokenUsingUvWithPermissions => 0x06,
            Self::GetUvRetries => 0x07,
            Self::GetPinUvAuthTokenUsingPinWithPermissions => 0x09,
            Self::Unknown(value) => value,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    keyringd_testkit::scenario! {
        group "ClientPinSubcommand::from_u64" {
            case "Preserves unassigned ClientPIN subcommand values" {
                why "The wire model retains unassigned subcommand bytes so diagnostics and proxying remain lossless when a newer authenticator introduces a value this crate does not know."
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#authnrClientPin-cmd-dfn"
                given "an unassigned ClientPIN subcommand value" {
                    let value = 0x08;
                }
                when "the value is decoded and encoded again" {
                    let subcommand = ClientPinSubcommand::from_u64(value);
                    let encoded = subcommand.as_u64();
                }
                then "the unknown value remains available for forward compatibility" {
                    assert_eq!(subcommand, ClientPinSubcommand::Unknown(value));
                    assert_eq!(encoded, value);
                }
            }
        }
    }
}
