use crate::webauthn::*;

keyringd_testkit::scenario! {
        case "Exposes every transport identifier defined by WebAuthn Level 3" {
            spec "https://www.w3.org/TR/webauthn-3/#enum-transport"

            given "the ordered transport identifiers in the WebAuthn Level 3 enumeration" {
                let expected = ["usb", "nfc", "ble", "smart-card", "hybrid", "internal"];
            }
            when "the named constructors create their transport identifiers" {
                let transports = [
                    AuthenticatorTransport::usb(),
                    AuthenticatorTransport::nfc(),
                    AuthenticatorTransport::ble(),
                    AuthenticatorTransport::smart_card(),
                    AuthenticatorTransport::hybrid(),
                    AuthenticatorTransport::internal(),
                ];
            }
            then "the identifiers exactly match the WebAuthn enumeration" {
                assert_eq!(
                    transports.map(|transport| transport.as_str().to_owned()),
                    expected
                );
            }
        }
        case "Retains an unknown transport identifier" {
            spec "https://www.w3.org/TR/webauthn-3/#enum-transport"

            given "a transport identifier introduced by a future specification" {
                let value = "future-transport";
            }
            when "the identifier is represented as an AuthenticatorTransport" {
                let transport = AuthenticatorTransport::new(value);
            }
            then "the original identifier remains available for forward compatibility" {
                assert_eq!(transport.as_str(), "future-transport");
            }
        }
}
