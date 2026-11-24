use crate::ctap::pin_uv::PinUvAuthProtocol;

keyringd_testkit::scenario! {
        case "Uses the numeric identifiers assigned to the PIN/UV auth protocols" {
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#pinProto1"
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#pinProto2"

            given "the two PIN/UV auth protocols implemented by the client" {
                let protocols = [PinUvAuthProtocol::One, PinUvAuthProtocol::Two];
            }
            when "their wire versions are requested" {
                let versions = protocols.map(PinUvAuthProtocol::version);
            }
            then "Protocol One and Protocol Two use identifiers one and two respectively" {
                assert_eq!(versions, [1, 2]);
            }
        }
}
