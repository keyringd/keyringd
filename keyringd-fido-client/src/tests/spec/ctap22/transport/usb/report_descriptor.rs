use crate::tests::support::transport_usb_report_report_descriptor::descriptor;
use crate::transport::usb::report::HidReportLayout;
use crate::transport::usb::report::parse_report_layout;

keyringd_testkit::scenario! {
    group "parse_report_layout" {
            case "Extract the size of the unnumbered report" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#usb-discovery"

                verify "Extract the size of the unnumbered report" {
                    assert_eq!(
                        parse_report_layout(&descriptor(32, 32, None)).unwrap(),
                        HidReportLayout {
                            report_id: None,
                            input_packet_size: 32,
                            output_packet_size: 32,
                        }
                    );
                    }

            }
            case "Extract size and ID for numbered report" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#usb-discovery"

                verify "Extract size and ID for numbered report" {
                    assert_eq!(
                        parse_report_layout(&descriptor(64, 64, Some(5))).unwrap(),
                        HidReportLayout {
                            report_id: Some(5),
                            input_packet_size: 64,
                            output_packet_size: 64,
                        }
                    );
                    }

            }
            case "Extract different Report Sizes at INPUT and OUTPUT" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#usb-discovery"

                verify "Extract different Report Sizes at INPUT and OUTPUT" {
                    assert_eq!(
                        parse_report_layout(&descriptor(32, 64, None)).unwrap(),
                        HidReportLayout {
                            report_id: None,
                            input_packet_size: 32,
                            output_packet_size: 64,
                        }
                    );
                    }

            }
    }
}
