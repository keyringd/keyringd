use crate::transport::usb::hid_api::CTAP_HID_USAGE;
use crate::transport::usb::hid_api::FIDO_USAGE_PAGE;
use crate::transport::usb::hid_api::is_ctap_hid_device;

keyringd_testkit::scenario! {
    case "Detect only HID Devices that match FIDO Usage Page and CTAPHID Usage" {
        spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#usb-discovery"

        given "Prepare a different identifier with a correct identifier and a USAGE page or USAGE" {
            let identifiers = [
                (FIDO_USAGE_PAGE, CTAP_HID_USAGE),
                (0x0001, CTAP_HID_USAGE),
                (FIDO_USAGE_PAGE, 0x02),
                (0x0001, 0x02),
            ];
        }
        when "Determine each HID device under the same conditions as the USB Discovery of Production" {
            let matches = identifiers.map(|(usage_page, usage)| {
                is_ctap_hid_device(usage_page, usage)
            });
        }
        then "Only 0xF1D0 and 0x01 pairs are selected as CTAPHID DEVICE" {
            assert_eq!(matches, [true, false, false, false]);
        }
    }
}
