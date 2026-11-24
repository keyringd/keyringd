use super::super::ProfileSetupProgress;
use crate::application::platform::windows::UiDriver;
use anyhow::{Context, Result};
use windows::Win32::UI::Accessibility::IUIAutomationElement;

pub(super) fn observe_profile_setup(
    driver: &UiDriver,
    window: &IUIAutomationElement,
) -> Result<ProfileSetupProgress> {
    let heading = if let Some(stage) = driver
        .visible_element_by_id(window, "profile-setup-progress-heading")
        .context("observe profile setup stage heading")?
    {
        Some(stage)
    } else {
        driver
            .visible_element_by_id(window, "profile-setup-title")
            .context("observe profile setup window heading")?
    };
    let security_key = driver
        .visible_element_by_id(window, "passkey-use-security-key")
        .context("observe profile setup security key button")?;
    let next = driver
        .visible_element_by_id(window, "profile-setup-not-found-continue")
        .context("observe profile setup missing-passkey Continue button")?;
    let mut texts = driver
        .visible_texts(window)
        .context("observe profile setup visible text")?;
    texts.extend(
        driver
            .visible_radio_labels(window)
            .context("observe profile setup visible radio choices")?,
    );
    unsafe {
        Ok(ProfileSetupProgress {
            heading: heading
                .map(|element| {
                    element
                        .CurrentName()
                        .map(|name| name.to_string())
                        .context("read profile setup heading Name")
                })
                .transpose()?
                .unwrap_or_default(),
            texts,
            pin_visible: false,
            security_key_enabled: security_key
                .map(|element| {
                    element
                        .CurrentIsEnabled()
                        .map(|enabled| enabled.as_bool())
                        .context("read profile setup security key IsEnabled")
                })
                .transpose()?,
            continue_enabled: next
                .map(|element| {
                    element
                        .CurrentIsEnabled()
                        .map(|enabled| enabled.as_bool())
                        .context("read profile setup Continue IsEnabled")
                })
                .transpose()?,
        })
    }
}
