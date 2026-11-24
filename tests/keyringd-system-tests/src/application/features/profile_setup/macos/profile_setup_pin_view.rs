use crate::application::platform::macos::UiDriver;
use anyhow::Result;
const EXPLANATION: &str = "profile-setup.prompt-explanation";
const PIN: &str = "profile-setup.pin";
const CONFIRM_PIN: &str = "profile-setup.confirm-pin";
const PIN_SUBMIT: &str = "profile-setup.pin-submit";
pub struct ProfileSetupPinView<'a> {
    pub(super) driver: UiDriver<'a>,
}
impl<'a> ProfileSetupPinView<'a> {
    pub fn submit(&self, pin: &str) -> Result<()> {
        self.driver
            .wait_until(PIN, "to become visible", |element| {
                element.hidden != Some(true)
            })?;
        self.driver.enter_text(PIN, pin)?;
        if self
            .driver
            .find_element(CONFIRM_PIN)?
            .is_some_and(|element| element.hidden != Some(true))
        {
            self.driver.enter_text(CONFIRM_PIN, pin)?;
        }
        self.driver
            .wait_until(PIN_SUBMIT, "to accept the validated PIN", |element| {
                element.enabled == Some(true)
            })?;
        self.driver.press(PIN_SUBMIT)
    }
}

impl super::super::ProfileSetupPinViewDriver for ProfileSetupPinView<'_> {
    fn visible(&self, _new_pin: bool) -> Result<bool> {
        Ok(self
            .driver
            .find_element(PIN)?
            .is_some_and(|field| field.hidden == Some(false) && field.enabled == Some(true)))
    }

    fn content(&self) -> Result<super::super::ProfileSetupPinContent> {
        let pin = self.driver.find_element(PIN)?;
        let confirmation = self
            .driver
            .find_element(CONFIRM_PIN)?
            .filter(|field| field.hidden != Some(true));
        Ok(super::super::ProfileSetupPinContent {
            texts: self
                .driver
                .find_element(EXPLANATION)?
                .and_then(|element| element.value)
                .into_iter()
                .collect(),
            pin_enabled: pin
                .is_some_and(|field| field.hidden != Some(true) && field.enabled == Some(true)),
            confirmation_enabled: confirmation.map(|field| field.enabled.unwrap_or(false)),
            submit_visible: self
                .driver
                .find_element(PIN_SUBMIT)?
                .is_some_and(|field| field.hidden != Some(true)),
        })
    }
    fn screenshot(&self) -> Result<std::path::PathBuf> {
        self.driver.capture(super::HEADING)
    }
    fn enter(&mut self, pin: &str, _new_pin: bool) -> Result<()> {
        self.submit(pin)
    }
}
