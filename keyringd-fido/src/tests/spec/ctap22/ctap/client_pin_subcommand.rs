use crate::ctap::ClientPinSubcommand;

keyringd_testkit::scenario! {
        case "Maps ClientPIN subcommands to their assigned values" {
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#authnrClientPin-cmd-dfn"

            given "all ClientPIN subcommands represented by this protocol crate" {
                let assigned = [
                    (ClientPinSubcommand::GetPinRetries, 0x01),
                    (ClientPinSubcommand::GetKeyAgreement, 0x02),
                    (ClientPinSubcommand::SetPin, 0x03),
                    (ClientPinSubcommand::ChangePin, 0x04),
                    (ClientPinSubcommand::GetPinToken, 0x05),
                    (
                        ClientPinSubcommand::GetPinUvAuthTokenUsingUvWithPermissions,
                        0x06,
                    ),
                    (ClientPinSubcommand::GetUvRetries, 0x07),
                    (
                        ClientPinSubcommand::GetPinUvAuthTokenUsingPinWithPermissions,
                        0x09,
                    ),
                ];
            }
            when "the subcommands are encoded and their assigned values are decoded" {
                let results = assigned.map(|(subcommand, value)| {
                    (subcommand.as_u64(), ClientPinSubcommand::from_u64(value))
                });
            }
            then "both directions use the subcommand numbers assigned by the specification" {
                for (index, (subcommand, expected)) in assigned.into_iter().enumerate() {
                    assert_eq!(results[index].0, expected);
                    assert_eq!(results[index].1, subcommand);
                }
            }

        }
}
