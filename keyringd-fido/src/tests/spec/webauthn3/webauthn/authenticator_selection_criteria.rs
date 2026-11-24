use crate::webauthn::*;

keyringd_testkit::scenario! {
        case "Applies the WebAuthn defaults for authenticator selection" {
            spec "https://www.w3.org/TR/webauthn-3/#dictdef-authenticatorselectioncriteria"

            given "no authenticator-selection members are supplied by the relying party" {
                let selection_input = ();
            }
            when "authenticator selection criteria are constructed without overrides" {
                let () = selection_input;
                let selection = AuthenticatorSelectionCriteria::default();
            }
            then "resident-key compatibility is false and user verification is preferred" {
                assert!(selection.authenticator_attachment().is_none());
                assert!(selection.resident_key().is_none());
                assert!(!selection.require_resident_key());
                assert_eq!(
                    selection.user_verification(),
                    &UserVerificationRequirement::Preferred
                );
            }
        }
}
