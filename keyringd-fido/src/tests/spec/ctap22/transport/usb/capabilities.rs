use crate::transport::usb::CtapHidCapabilities;

keyringd_testkit::scenario! {
    group "CtapHidCapabilities bit assignments" {
            case "Uses the capability bits assigned by CTAPHID INIT" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#usb-hid-init"

                given "the WINK, CBOR, and NMSG capability constants" {
                    let capabilities = [
                        (CtapHidCapabilities::WINK, 0x01),
                        (CtapHidCapabilities::CBOR, 0x04),
                        (CtapHidCapabilities::NO_MSG, 0x08),
                    ];
                }
                when "their raw capability bytes are read" {
                    let bits = capabilities.map(|(capability, _)| capability.bits());
                }
                then "each constant uses its assigned bit" {
                    assert_eq!(bits, [0x01, 0x04, 0x08]);
                }
            }
    }
}
