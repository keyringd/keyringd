use std::time::SystemTime;

use anyhow::{Context, Result, ensure};
use plist::{Dictionary, Value};

use super::macos_bundle::BUNDLE_ID;

pub(crate) fn entitlements(
    profile: &Value,
    certificate: &[u8],
    team: &str,
    now: SystemTime,
) -> Result<Value> {
    let profile = profile
        .as_dictionary()
        .context("provisioning profile must be a dictionary")?;
    ensure!(
        array(profile, "TeamIdentifier")?
            .iter()
            .any(|value| value.as_string() == Some(team)),
        "provisioning profile belongs to a different team"
    );
    let expiration: SystemTime = profile
        .get("ExpirationDate")
        .and_then(Value::as_date)
        .context("missing profile expiration")?
        .into();
    ensure!(expiration > now, "provisioning profile has expired");
    ensure!(
        profile
            .get("ProvisionsAllDevices")
            .and_then(Value::as_boolean)
            == Some(true),
        "a Developer ID distribution profile is required"
    );
    ensure!(
        profile
            .get("ProvisionedDevices")
            .is_none_or(|value| value.as_array().is_some_and(Vec::is_empty)),
        "device-limited profiles cannot be used for releases"
    );
    ensure!(
        array(profile, "DeveloperCertificates")?
            .iter()
            .any(|value| value.as_data() == Some(certificate)),
        "provisioning profile does not authorize the signing certificate"
    );
    let allowed = profile
        .get("Entitlements")
        .and_then(Value::as_dictionary)
        .context("missing profile entitlements")?;
    for key in ["get-task-allow", "com.apple.security.get-task-allow"] {
        ensure!(
            allowed
                .get(key)
                .is_none_or(|value| value.as_boolean() == Some(false)),
            "development provisioning profiles cannot be used for releases"
        );
    }
    let prefix = array(profile, "ApplicationIdentifierPrefix")?
        .first()
        .and_then(Value::as_string)
        .filter(|prefix| !prefix.is_empty())
        .context("missing App ID prefix")?;
    let app_id = format!("{prefix}.{BUNDLE_ID}");
    ensure!(
        allowed
            .get("com.apple.application-identifier")
            .and_then(Value::as_string)
            .is_some_and(|pattern| authorizes(pattern, &app_id)),
        "provisioning profile does not authorize the keyringd App ID"
    );
    ensure!(
        allowed
            .get("com.apple.developer.team-identifier")
            .and_then(Value::as_string)
            == Some(team),
        "provisioning profile does not authorize the Team ID"
    );
    ensure!(
        array(allowed, "keychain-access-groups")?
            .iter()
            .any(|group| group
                .as_string()
                .is_some_and(|pattern| authorizes(pattern, &app_id))),
        "provisioning profile does not authorize the keyringd Keychain access group"
    );
    let mut entitlements = Dictionary::new();
    entitlements.insert(
        "com.apple.application-identifier".into(),
        app_id.clone().into(),
    );
    entitlements.insert("com.apple.developer.team-identifier".into(), team.into());
    entitlements.insert(
        "keychain-access-groups".into(),
        Value::Array(vec![app_id.into()]),
    );
    Ok(Value::Dictionary(entitlements))
}

fn array<'a>(dictionary: &'a Dictionary, key: &str) -> Result<&'a Vec<Value>> {
    dictionary
        .get(key)
        .and_then(Value::as_array)
        .with_context(|| format!("missing or invalid profile {key}"))
}

fn authorizes(pattern: &str, app_id: &str) -> bool {
    // Apple's App ID / keychain grants are exact identifiers or a prefix ending in '*'.
    match pattern.strip_suffix('*') {
        Some(prefix) => !prefix.contains('*') && app_id.starts_with(prefix),
        None => pattern == app_id,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn now() -> SystemTime {
        SystemTime::UNIX_EPOCH + Duration::from_secs(1_700_000_000)
    }

    fn profile() -> Value {
        let mut allowed = Dictionary::new();
        allowed.insert(
            "com.apple.application-identifier".into(),
            "PREFIX.com.keyringd.*".into(),
        );
        allowed.insert("com.apple.developer.team-identifier".into(), "TEAM".into());
        allowed.insert(
            "keychain-access-groups".into(),
            Value::Array(vec!["PREFIX.*".into()]),
        );
        allowed.insert("unneeded-entitlement".into(), true.into());
        let mut profile = Dictionary::new();
        profile.insert("TeamIdentifier".into(), Value::Array(vec!["TEAM".into()]));
        profile.insert(
            "ApplicationIdentifierPrefix".into(),
            Value::Array(vec!["PREFIX".into()]),
        );
        profile.insert(
            "ExpirationDate".into(),
            Value::Date((now() + Duration::from_secs(1)).into()),
        );
        profile.insert("ProvisionsAllDevices".into(), true.into());
        profile.insert(
            "DeveloperCertificates".into(),
            Value::Array(vec![Value::Data(b"certificate".to_vec())]),
        );
        profile.insert("Entitlements".into(), Value::Dictionary(allowed));
        Value::Dictionary(profile)
    }

    keyringd_testkit::scenario! {
        group "entitlements" {
            case "Resolves a prefix distinct from team and emits only required entitlements" {
                verify "use the profile's distinct application prefix and include only required entitlements" {
                    let result = entitlements(&profile(), b"certificate", "TEAM", now()).unwrap();
                    let result = result.as_dictionary().unwrap();
                    assert_eq!(result.len(), 3);
                    assert_eq!(
                        result["com.apple.application-identifier"].as_string(),
                        Some("PREFIX.com.keyringd.Keyringd")
                    );
                    assert_eq!(
                        result["keychain-access-groups"].as_array().unwrap()[0].as_string(),
                        Some("PREFIX.com.keyringd.Keyringd")
                    );
                    assert!(entitlements(&profile(), b"other-certificate", "TEAM", now()).is_err());
                    assert!(entitlements(&profile(), b"certificate", "OTHER", now()).is_err());
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "entitlements" {
            case "Rejects expired device limited and malformed profiles" {
                verify "fail provisioning validation for invalid dates, device restrictions, or malformed input" {
                    for (key, value) in [
                        ("ExpirationDate", Value::Date(now().into())),
                        ("ExpirationDate", Value::String("invalid".into())),
                        ("ProvisionsAllDevices", false.into()),
                        ("ProvisionedDevices", Value::Array(vec!["device".into()])),
                        ("ApplicationIdentifierPrefix", Value::Array(vec![])),
                        ("DeveloperCertificates", Value::Array(vec![])),
                    ] {
                        let mut profile = profile();
                        profile
                            .as_dictionary_mut()
                            .unwrap()
                            .insert(key.into(), value);
                        assert!(
                            entitlements(&profile, b"certificate", "TEAM", now()).is_err(),
                            "{key}"
                        );
                    }
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "entitlements" {
            case "Rejects development and unauthorized entitlements" {
                verify "fail provisioning validation for development grants or unapproved entitlements" {
                    for (key, value) in [
                        ("get-task-allow", true.into()),
                        ("com.apple.security.get-task-allow", true.into()),
                        ("com.apple.application-identifier", "OTHER.*".into()),
                        ("com.apple.developer.team-identifier", "OTHER".into()),
                        (
                            "keychain-access-groups",
                            Value::Array(vec!["OTHER.*".into()]),
                        ),
                    ] {
                        let mut profile = profile();
                        profile
                            .as_dictionary_mut()
                            .unwrap()
                            .get_mut("Entitlements")
                            .unwrap()
                            .as_dictionary_mut()
                            .unwrap()
                            .insert(key.into(), value);
                        assert!(
                            entitlements(&profile, b"certificate", "TEAM", now()).is_err(),
                            "{key}"
                        );
                    }
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "authorizes" {
            case "Exact grants do not authorize another identifier" {
                verify "reject an entitlement that attempts to reuse an exact grant for another identifier" {
                    assert!(authorizes(
                        "PREFIX.com.keyringd.Keyringd",
                        "PREFIX.com.keyringd.Keyringd"
                    ));
                    assert!(!authorizes(
                        "PREFIX.com.keyringd.Keyringd",
                        "PREFIX.com.keyringd.Other"
                    ));
                    assert!(!authorizes(
                        "PREFIX.*.*",
                        "PREFIX.com.keyringd.Keyringd"
                    ));
                }
            }
        }
    }
}
