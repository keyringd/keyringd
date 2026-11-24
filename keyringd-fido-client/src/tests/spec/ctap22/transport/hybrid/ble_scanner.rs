use crate::transport::hybrid::ble::FIDO_HYBRID_SERVICE;
use crate::transport::hybrid::ble::GOOGLE_CABLE_SERVICE;

keyringd_testkit::scenario! {
        case "Service Identifier matches the Hybrid specification" {
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#hybrid-state-assisted"

            given "Prepare two BLESERVICE UUIDs for use by Hybrid Transport" {
                let fido = FIDO_HYBRID_SERVICE;
                let cable = GOOGLE_CABLE_SERVICE;
        }
        when "Convert UUID to String Representation" {
            let fido_text = fido.to_string();
            let cable_text = cable.to_string();
        }
        then "Match the FIDO Hybrid Specifications and the Google Cable Quota" {
            assert_eq!(
                fido_text,
                "0000fff9-0000-1000-8000-00805f9b34fb"
            );
            assert_eq!(
                cable_text,
                "0000fde2-0000-1000-8000-00805f9b34fb"
            );
        }

        }
}
