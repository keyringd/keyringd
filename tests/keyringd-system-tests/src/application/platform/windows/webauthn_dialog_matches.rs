/// CNG can also mention keyringd, but its key description identifies a
/// different operation. Never select it as a WebAuthn prompt.
pub(crate) fn webauthn_dialog_matches(names: &[String]) -> bool {
    !names
        .iter()
        .any(|name| name.contains("TPM-protected SSH key"))
        && names
            .iter()
            .any(|name| name.to_ascii_lowercase().contains("keyringd"))
}

#[cfg(test)]
mod tests {
    use super::*;

    keyringd_testkit::scenario! {
        group "webauthn_dialog_matches" {
            case "Distinguishes relying party from cng and unrelated requests" {
                verify "match the intended WebAuthn relying party and ignore CNG or unrelated prompts" {
                    assert!(webauthn_dialog_matches(&[
                        "Sign in to ssh:keyringd".into()
                    ]));
                    assert!(webauthn_dialog_matches(&[
                        "This request comes from Keyringd".into()
                    ]));
                    assert!(!webauthn_dialog_matches(&["Sign in to example.com".into()]));
                    assert!(!webauthn_dialog_matches(&[
                        "Keyringd".into(),
                        "TPM-protected SSH key for the \"test\" profile".into(),
                    ]));
                }
            }
        }
    }
}
