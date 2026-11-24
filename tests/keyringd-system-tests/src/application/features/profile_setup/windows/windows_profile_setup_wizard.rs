use crate::application::features::profile_setup::ProfileSetupWizardDriver;
use crate::application::platform::windows::UiDriver;
use anyhow::{Context, Result};

pub struct WindowsProfileSetupWizard<'a> {
    driver: &'a UiDriver,
    pid: u32,
}
impl<'a> WindowsProfileSetupWizard<'a> {
    pub(crate) fn new(driver: &'a UiDriver, pid: u32) -> Self {
        Self { driver, pid }
    }
    fn window(&self) -> Result<windows::Win32::UI::Accessibility::IUIAutomationElement> {
        self.driver.window_named(self.pid, "Add SSH profile")
    }
}
impl ProfileSetupWizardDriver for WindowsProfileSetupWizard<'_> {
    fn observe(&mut self) -> Result<super::super::ProfileSetupProgress> {
        let Some(handle) = crate::application::platform::windows::window_locator::named_window(
            self.pid,
            "Add SSH profile",
        )
        .context("locate profile setup native window for progress observation")?
        else {
            return Ok(super::super::ProfileSetupProgress {
                heading: String::new(),
                texts: Vec::new(),
                security_key_enabled: None,
                pin_visible: false,
                continue_enabled: None,
            });
        };
        let window = self
            .driver
            .element_from_handle(handle.0 as usize)
            .context("obtain profile setup UIA root for progress observation")?;
        super::observe_profile_setup(self.driver, &window)
            .context("observe Windows profile setup progress")
    }

    type ProfileFormPage<'a>
        = super::ProfileFormPage<'a>
    where
        Self: 'a;
    fn profile_form_page(&mut self) -> Result<Self::ProfileFormPage<'_>> {
        let window = self.window()?;
        self.driver.wait_visible(&window, "profile-setup-name")?;
        Ok(super::ProfileFormPage {
            driver: self.driver,
            window,
        })
    }
    type PasskeySelectionPage<'a>
        = super::PasskeySelectionPage<'a>
    where
        Self: 'a;
    fn passkey_selection_page(&mut self) -> Result<Self::PasskeySelectionPage<'_>> {
        let window = self.window()?;
        self.driver
            .wait_visible(&window, "profile-setup-passkey-candidates")?;
        Ok(super::PasskeySelectionPage {
            driver: self.driver,
            window,
        })
    }
    type PasskeyNotFoundPage<'a>
        = super::PasskeyNotFoundPage<'a>
    where
        Self: 'a;
    fn passkey_not_found_page(&mut self) -> Result<Self::PasskeyNotFoundPage<'_>> {
        let window = self.window()?;
        self.driver
            .wait_visible(&window, "profile-setup-not-found-continue")?;
        Ok(super::PasskeyNotFoundPage {
            driver: self.driver,
            window,
        })
    }
    type InteractionPage<'a>
        = super::ProfileSetupInteractionPage<'a>
    where
        Self: 'a;
    fn interaction_page(&mut self) -> Result<Self::InteractionPage<'_>> {
        let window = self.window()?;
        self.driver
            .wait_visible(&window, "profile-setup-progress-heading")?;
        Ok(super::ProfileSetupInteractionPage {
            driver: self.driver,
            window,
        })
    }
    type CompletionPage<'a>
        = super::CompletionPage<'a>
    where
        Self: 'a;
    fn completion_page(&mut self) -> Result<Self::CompletionPage<'_>> {
        let window = self.window()?;
        self.driver.wait_visible(&window, "profile-setup-done")?;
        Ok(super::CompletionPage {
            driver: self.driver,
            window,
            pid: self.pid,
        })
    }

    fn back(&mut self) -> Result<()> {
        self.driver.invoke_named(&self.window()?, "Back")
    }
}
