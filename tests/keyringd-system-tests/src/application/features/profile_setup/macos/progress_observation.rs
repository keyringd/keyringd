use super::HEADING;
use crate::application::features::profile_setup::ProfileSetupProgress;
use crate::application::platform::macos::UiDriver;
use anyhow::Result;

pub(super) fn observe_progress(driver: &UiDriver<'_>) -> Result<ProfileSetupProgress> {
    let elements = driver.elements()?;
    let visible = elements
        .iter()
        .filter(|element| element.hidden != Some(true))
        .collect::<Vec<_>>();
    let heading = visible
        .iter()
        .find(|element| element.identifier.as_deref() == Some(HEADING))
        .and_then(|element| element.value.clone())
        .unwrap_or_default();
    let texts = visible
        .iter()
        .flat_map(|element| element.value.iter().chain(element.title.iter()))
        .filter(|text| !text.is_empty())
        .cloned()
        .collect();
    let security_key_enabled = visible
        .iter()
        .find(|element| element.identifier.as_deref() == Some("profile-setup.use-security-key"))
        .and_then(|element| element.enabled);
    let continue_enabled = visible
        .iter()
        .find(|element| element.identifier.as_deref() == Some("profile-setup.not-found-continue"))
        .and_then(|element| element.enabled);
    Ok(super::super::ProfileSetupProgress {
        heading,
        texts,
        security_key_enabled,
        continue_enabled,
        pin_visible: visible.iter().any(|element| {
            element.identifier.as_deref() == Some("profile-setup.pin")
                && element.hidden == Some(false)
        }),
    })
}
