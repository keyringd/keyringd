use super::{
    CompletionPage, PasskeyNotFoundPage, PasskeySelectionPage, ProfileFormPage,
    ProfileSetupInteractionPage,
};
use crate::application::features::profile_setup::{ProfileSetupProgress, ProfileSetupWizardDriver};
use crate::application::platform::macos::UiDriver;
use anyhow::Result;

pub(crate) const HEADING: &str = "profile-setup.heading";

pub struct MacosProfileSetupWizard<'a> {
    driver: UiDriver<'a>,
}
impl<'a> MacosProfileSetupWizard<'a> {
    pub(crate) fn new(driver: &UiDriver<'a>) -> Self {
        Self {
            driver: driver.in_window(HEADING),
        }
    }
    pub fn wait_for(driver: &UiDriver<'a>) -> Result<Self> {
        let wizard = Self::new(driver);
        wizard.driver.wait_value(HEADING, "Create SSH Profile")?;
        Ok(wizard)
    }
    pub(crate) fn current(driver: &UiDriver<'a>) -> Result<Self> {
        let wizard = Self::new(driver);
        wizard.driver.wait_for(HEADING)?;
        Ok(wizard)
    }
    pub fn capture_screenshot(&self, name: &str) -> anyhow::Result<()> {
        keyringd_testkit::record_image(name, self.screenshot()?)
    }
    pub fn screenshot(&self) -> Result<std::path::PathBuf> {
        self.driver.capture(HEADING)
    }
}
impl ProfileSetupWizardDriver for MacosProfileSetupWizard<'_> {
    type ProfileFormPage<'a>
        = ProfileFormPage<'a>
    where
        Self: 'a;
    type PasskeySelectionPage<'a>
        = PasskeySelectionPage<'a>
    where
        Self: 'a;
    type PasskeyNotFoundPage<'a>
        = PasskeyNotFoundPage<'a>
    where
        Self: 'a;
    type InteractionPage<'a>
        = ProfileSetupInteractionPage<'a>
    where
        Self: 'a;
    type CompletionPage<'a>
        = CompletionPage<'a>
    where
        Self: 'a;

    fn back(&mut self) -> Result<()> {
        self.driver.press("profile-setup.back")
    }
    fn observe(&mut self) -> Result<ProfileSetupProgress> {
        super::observe_progress(&self.driver)
    }
    fn profile_form_page(&mut self) -> Result<Self::ProfileFormPage<'_>> {
        self.driver.wait_for("profile-setup.name")?;
        Ok(ProfileFormPage {
            driver: self.driver.in_window(HEADING),
        })
    }
    fn passkey_selection_page(&mut self) -> Result<Self::PasskeySelectionPage<'_>> {
        self.driver.wait_value(HEADING, "Choose Passkey")?;
        self.driver.wait_for("profile-setup.credential-selection")?;
        Ok(PasskeySelectionPage {
            driver: self.driver.in_window(HEADING),
        })
    }
    fn passkey_not_found_page(&mut self) -> Result<Self::PasskeyNotFoundPage<'_>> {
        self.driver.wait_value(HEADING, "Passkey Not Found")?;
        self.driver.wait_for("profile-setup.try-another-device")?;
        self.driver.wait_for("profile-setup.not-found-continue")?;
        Ok(PasskeyNotFoundPage {
            driver: self.driver.in_window(HEADING),
        })
    }
    fn interaction_page(&mut self) -> Result<Self::InteractionPage<'_>> {
        self.driver
            .wait_for_observation("the profile setup interaction page to appear", || {
                let elements = self.driver.elements()?;
                let visible = elements
                    .iter()
                    .filter(|element| element.hidden == Some(false))
                    .collect::<Vec<_>>();
                let appeared = visible.iter().any(|element| {
                    matches!(
                        element.identifier.as_deref(),
                        Some(
                            "key-operation.qr"
                                | "profile-setup.prompt-title"
                                | "profile-setup.waiting"
                                | "profile-setup.pin"
                        )
                    )
                });
                Ok((
                    appeared.then_some(()),
                    format!("visible interaction candidates: {visible:?}"),
                ))
            })?;
        Ok(ProfileSetupInteractionPage {
            driver: self.driver.in_window(HEADING),
        })
    }
    fn completion_page(&mut self) -> Result<Self::CompletionPage<'_>> {
        self.driver.wait_for("profile-setup.done")?;
        Ok(CompletionPage {
            driver: self.driver.in_window(HEADING),
        })
    }
}
