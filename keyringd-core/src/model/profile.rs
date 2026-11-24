use crate::model::{ProfileId, device_key::DeviceKey, passkey::Passkey};

#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Profile {
    #[serde(deserialize_with = "deserialize_profile_id")]
    id: ProfileId,
    #[serde(deserialize_with = "deserialize_display_name")]
    display_name: String,
    enabled: bool,
    device_key: DeviceKey,
    passkey: Passkey,
}

impl Profile {
    pub(crate) fn validate(&self) -> Result<(), &'static str> {
        validate_id(&self.id)?;
        Self::validate_display_name(&self.display_name)
    }

    pub(crate) fn new(
        id: ProfileId,
        display_name: String,
        enabled: bool,
        device_key: DeviceKey,
        passkey: Passkey,
    ) -> Result<Self, &'static str> {
        let profile = Self {
            id,
            display_name,
            enabled,
            device_key,
            passkey,
        };
        profile.validate()?;
        Ok(profile)
    }

    pub fn id(&self) -> &ProfileId {
        &self.id
    }

    pub fn display_name(&self) -> &str {
        &self.display_name
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn device_key(&self) -> &DeviceKey {
        &self.device_key
    }

    pub fn passkey(&self) -> &Passkey {
        &self.passkey
    }

    pub(crate) fn validate_display_name(name: &str) -> Result<(), &'static str> {
        if name.trim() != name || name.is_empty() || name.chars().count() > 80 {
            Err("profile name must contain 1 to 80 characters without surrounding whitespace")
        } else if name
            .chars()
            .any(|character| character.is_control() || matches!(character, '\u{2028}' | '\u{2029}'))
        {
            Err("profile name must not contain control characters or line separators")
        } else {
            Ok(())
        }
    }

    pub(crate) fn rename(&mut self, display_name: String) -> Result<(), &'static str> {
        Self::validate_display_name(&display_name)?;
        self.display_name = display_name;
        Ok(())
    }

    pub(crate) fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub(crate) fn replace_device_key(&mut self, device_key: DeviceKey) {
        self.device_key = device_key;
    }
}

fn validate_id(id: &ProfileId) -> Result<(), &'static str> {
    if id.is_valid() {
        Ok(())
    } else {
        Err("profile contains an invalid ID")
    }
}
fn deserialize_profile_id<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<ProfileId, D::Error> {
    let id = <ProfileId as serde::Deserialize>::deserialize(deserializer)?;
    validate_id(&id).map_err(serde::de::Error::custom)?;
    Ok(id)
}
fn deserialize_display_name<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<String, D::Error> {
    let name = <String as serde::Deserialize>::deserialize(deserializer)?;
    Profile::validate_display_name(&name).map_err(serde::de::Error::custom)?;
    Ok(name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::ModelProfileFixture;

    keyringd_testkit::scenario! {
        group "Profile::new" {
            case "Applies the same name boundaries to construction renaming and deserialization" {
                verify "enforce the same display-name limits at all three entry points" {
                    for (name, valid) in [
                        ("a".to_owned(), true),
                        ("a".repeat(80), true),
                        ("é".repeat(80), true),
                        ("a".repeat(81), false),
                        ("é".repeat(81), false),
                        ("".to_owned(), false),
                        (" ".to_owned(), false),
                        (" Work".to_owned(), false),
                        ("Work\n".to_owned(), false),
                        ("\u{2003}Work".to_owned(), false),
                        ("Work Team".to_owned(), true),
                        ("Work\nTeam".to_owned(), false),
                        ("Work\rTeam".to_owned(), false),
                        ("Work\tTeam".to_owned(), false),
                        ("Work\0Team".to_owned(), false),
                        ("Work\u{1b}Team".to_owned(), false),
                        ("Work\u{7f}Team".to_owned(), false),
                        ("Work\u{85}Team".to_owned(), false),
                        ("Work\u{2028}Team".to_owned(), false),
                        ("Work\u{2029}Team".to_owned(), false),
                    ] {
                        let mut profile = ModelProfileFixture::default().build();
                        let original_name = profile.display_name().to_owned();
                        assert_eq!(Profile::new(
                            profile.id().clone(), name.clone(), profile.is_enabled(),
                            profile.device_key().clone(), profile.passkey().clone(),
                        ).is_ok(), valid, "{name:?}");
                        assert_eq!(profile.rename(name.clone()).is_ok(), valid, "{name:?}");
                        assert_eq!(profile.display_name(), if valid { &name } else { &original_name });
                        assert!(profile.validate().is_ok());
                        let mut encoded = serde_json::to_value(&profile).unwrap();
                        encoded["displayName"] = serde_json::json!(name);
                        assert_eq!(
                            serde_json::from_value::<Profile>(encoded).is_ok(),
                            valid,
                            "{name:?}"
                        );
                    }
                }
            }

            case "Rejects invalid IDs during construction and deserialization" {
                verify "reject empty IDs and IDs containing spaces, slashes, or non-ASCII characters" {
                    for value in ["", "with space", "bad/id", "café"] {
                        let profile = ModelProfileFixture::default().build();
                        assert!(Profile::new(
                            ProfileId::from_stored(value.into()), profile.display_name().to_owned(),
                            profile.is_enabled(), profile.device_key().clone(), profile.passkey().clone(),
                        ).is_err(), "{value:?}");
                        let mut encoded = serde_json::to_value(profile).unwrap();
                        encoded["id"] = serde_json::json!(value);
                        assert!(serde_json::from_value::<Profile>(encoded).is_err(), "{value:?}");
                    }
                }
            }
        }
        group "Profile::replace_device_key" {
            case "Keeps profile identity and Passkey when the Device Key is replaced" {
                verify "preserve the profile ID and serialized Passkey through key replacement" {
                    let mut profile = ModelProfileFixture::default().build();
                    let original_id = profile.id().clone();
                    let original_passkey = serde_json::to_value(profile.passkey()).unwrap();
                    profile.rename("Renamed".into()).unwrap();
                    profile.set_enabled(false);
                    let replacement = ModelProfileFixture {
                        key_seed: 12,
                        ..Default::default()
                    }
                    .build();
                    profile.replace_device_key(replacement.device_key().clone());
                    assert_eq!(profile.id(), &original_id);
                    assert_eq!(
                        serde_json::to_value(profile.passkey()).unwrap(),
                        original_passkey
                    );
                    assert_eq!(profile.display_name(), "Renamed");
                    assert!(!profile.is_enabled());
                    assert_eq!(
                        profile.device_key().certificate().public_key(),
                        replacement.device_key().certificate().public_key()
                    );
                    assert!(profile.validate().is_ok());
                }
            }
        }
    }
}
