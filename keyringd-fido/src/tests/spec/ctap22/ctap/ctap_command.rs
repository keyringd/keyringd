use crate::CtapCommand;

keyringd_testkit::scenario! {
        case "Maps CTAP authenticator commands to their assigned command bytes" {
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#commands"

            given "all non-prototype authenticator commands assigned by CTAP 2.2" {
                let assigned = [
                    (CtapCommand::MakeCredential, 0x01),
                    (CtapCommand::GetAssertion, 0x02),
                    (CtapCommand::GetInfo, 0x04),
                    (CtapCommand::ClientPin, 0x06),
                    (CtapCommand::Reset, 0x07),
                    (CtapCommand::GetNextAssertion, 0x08),
                    (CtapCommand::BioEnrollment, 0x09),
                    (CtapCommand::CredentialManagement, 0x0a),
                    (CtapCommand::Selection, 0x0b),
                    (CtapCommand::LargeBlobs, 0x0c),
                    (CtapCommand::Config, 0x0d),
                ];
            }
            when "the commands are encoded and their assigned bytes are decoded" {
                let results = assigned.map(|(command, byte)| {
                    (command.as_byte(), CtapCommand::from_byte(byte))
                });
            }
            then "both directions use the command numbers assigned by the specification" {
                for (index, (command, expected)) in assigned.into_iter().enumerate() {
                    assert_eq!(results[index].0, expected);
                    assert_eq!(results[index].1, command);
                }
            }

        }
}
