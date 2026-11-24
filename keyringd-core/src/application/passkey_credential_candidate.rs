use crate::model::passkey::CredentialId;
use base64::Engine;
use keyringd_fido_client::AssertionResponse;
use std::collections::HashSet;

/// A recoverable Passkey credential offered by an authenticator.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PasskeyCredentialCandidate {
    credential_id: CredentialId,
    label: String,
}

impl PasskeyCredentialCandidate {
    pub(crate) fn from_assertion(response: &AssertionResponse) -> Self {
        let credential_id = CredentialId::new(response.credential().id().to_vec());
        let name = response
            .user()
            .and_then(|user| user.display_name().or_else(|| user.name()))
            .filter(|name| !name.trim().is_empty())
            .unwrap_or("Passkey");
        Self {
            credential_id,
            label: name.to_owned(),
        }
    }

    pub(crate) fn make_labels_unique(candidates: &mut [Self]) {
        // Reserve original labels before generating replacements.
        let mut used_labels = HashSet::with_capacity(candidates.len());
        let mut repeated_labels = HashSet::new();
        for candidate in candidates.iter() {
            if !used_labels.insert(candidate.label.clone()) {
                repeated_labels.insert(candidate.label.clone());
            }
        }
        for candidate in candidates {
            if !repeated_labels.contains(&candidate.label) {
                continue;
            }
            let id = base64::engine::general_purpose::URL_SAFE_NO_PAD
                .encode(candidate.credential_id.as_bytes());
            let base_label = format!("{} (ID {})", candidate.label, &id[..id.len().min(12)]);
            let mut label = base_label.clone();
            let mut suffix = 2usize;
            while used_labels.contains(&label) {
                label = format!("{base_label} ({suffix})");
                suffix += 1;
            }
            used_labels.insert(label.clone());
            candidate.label = label;
        }
    }

    pub fn credential_id(&self) -> &CredentialId {
        &self.credential_id
    }

    pub fn label(&self) -> &str {
        &self.label
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use keyringd_fido::ctap::AuthenticatorGetAssertionResponse;

    keyringd_testkit::scenario! {
        group "PasskeyCredentialCandidate::from_assertion" {
            case "Chooses the authenticator display name then name then the default label" {
                verify "use the account name when no display name is present and Passkey when neither is usable" {
                    use keyringd_fido::AuthenticatorData;
                    use keyringd_fido::webauthn::{
                        PublicKeyCredentialDescriptor, PublicKeyCredentialUserEntity,
                    };
                    for (display, name, expected) in [
                        (Some("Display"), Some("Account"), "Display"),
                        (None, Some("Account"), "Account"),
                        (None, None, "Passkey"),
                        (Some("   "), None, "Passkey"),
                    ] {
                        let mut user = PublicKeyCredentialUserEntity::from_id(vec![7; 32]);
                        if let Some(name) = name {
                            user = user.with_name(name);
                        }
                        if let Some(display) = display {
                            user = user.with_display_name(display);
                        }
                        let response = AuthenticatorGetAssertionResponse::new(Some(PublicKeyCredentialDescriptor::new(
                                keyringd_fido::webauthn::PublicKeyCredentialType::PublicKey,
                                vec![1, 2],
                            )),
                            AuthenticatorData::from_bytes(&[0; 37]).unwrap(),
                            vec![3],
                        )
                        .with_user(user);
                        let candidate = PasskeyCredentialCandidate::from_assertion(&AssertionResponse::from_ctap(response, None).unwrap());
                        assert_eq!(candidate.credential_id().as_bytes(), &[1, 2]);
                        assert_eq!(candidate.label(), expected);
                    }
                    let response = AuthenticatorGetAssertionResponse::new(Some(PublicKeyCredentialDescriptor::new(
                            keyringd_fido::webauthn::PublicKeyCredentialType::PublicKey,
                            vec![1],
                        )),
                        AuthenticatorData::from_bytes(&[0; 37]).unwrap(),
                        vec![3],
                    );
                    assert_eq!(
                        PasskeyCredentialCandidate::from_assertion(&AssertionResponse::from_ctap(response, None).unwrap()).label(),
                        "Passkey"
                    );
                }
            }
        }
        group "PasskeyCredentialCandidate::make_labels_unique" {
            case "Adds credential ID suffixes only to repeated labels while preserving candidate order" {
                verify "append credential ID prefixes to repeated labels in their original order" {
                    let mut candidates = vec![
                        PasskeyCredentialCandidate {
                            credential_id: CredentialId::new(vec![1]),
                            label: "Same".into(),
                        },
                        PasskeyCredentialCandidate {
                            credential_id: CredentialId::new(vec![9]),
                            label: "Unique".into(),
                        },
                        PasskeyCredentialCandidate {
                            credential_id: CredentialId::new(vec![2]),
                            label: "Same".into(),
                        },
                    ];
                    PasskeyCredentialCandidate::make_labels_unique(&mut candidates);
                    assert_eq!(
                        candidates.iter().map(|c| c.label()).collect::<Vec<_>>(),
                        ["Same (ID AQ)", "Unique", "Same (ID Ag)"]
                    );
                    assert_eq!(
                        candidates
                            .iter()
                            .map(|c| c.credential_id().as_bytes())
                            .collect::<Vec<_>>(),
                        [vec![1], vec![9], vec![2]]
                            .iter()
                            .map(Vec::as_slice)
                            .collect::<Vec<_>>()
                    );
                    let mut empty = [];
                    PasskeyCredentialCandidate::make_labels_unique(&mut empty);
                }
            }
            case "Keeps labels unique when ID prefixes and existing labels collide" {
                verify "number conflicting generated labels while preserving the original unique labels" {
                    let mut second_id = vec![0; 10];
                    second_id[9] = 1;
                    let mut candidates = [
                        PasskeyCredentialCandidate {
                            credential_id: CredentialId::new(vec![0; 10]),
                            label: "Work".into(),
                        },
                        PasskeyCredentialCandidate {
                            credential_id: CredentialId::new(second_id),
                            label: "Work".into(),
                        },
                        PasskeyCredentialCandidate {
                            credential_id: CredentialId::new(vec![1]),
                            label: "Work (ID AAAAAAAAAAAA)".into(),
                        },
                        PasskeyCredentialCandidate {
                            credential_id: CredentialId::new(vec![2]),
                            label: "Work (ID AAAAAAAAAAAA) (2)".into(),
                        },
                    ];
                    PasskeyCredentialCandidate::make_labels_unique(&mut candidates);
                    assert_eq!(
                        candidates.each_ref().map(PasskeyCredentialCandidate::label),
                        [
                            "Work (ID AAAAAAAAAAAA) (3)",
                            "Work (ID AAAAAAAAAAAA) (4)",
                            "Work (ID AAAAAAAAAAAA)",
                            "Work (ID AAAAAAAAAAAA) (2)",
                        ]
                    );
                }
            }
        }
    }
}
